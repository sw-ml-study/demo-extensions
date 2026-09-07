use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use mlpl_extension_sdk::{HandleError, HandleRegistry, NativeHandle, OwnedError, Value};
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Connection, OpenFlags, params_from_iter};

const EXTENSION_ID: u64 = 0x53_51_4c_49_54_45_5f_31;
const CONNECTION_TYPE: u64 = 1;
const MAX_CONNECTIONS: usize = 32;
const MAX_BUSY_TIMEOUT_MS: i64 = 60_000;
const MAX_ROWS: i64 = 10_000;
const MAX_RESULT_BYTES: i64 = 16 * 1024 * 1024;
const MAX_SQL_BYTES: usize = 64 * 1024;
const MAX_PARAMETERS: usize = 999;
const MAX_COLUMNS: usize = 256;

thread_local! {
    static CONNECTIONS: RefCell<HandleRegistry> = const {
        RefCell::new(HandleRegistry::with_limits(
            EXTENSION_ID,
            MAX_CONNECTIONS,
            u32::MAX,
        ))
    };
}

struct Database {
    connection: Connection,
    max_rows: usize,
    max_result_bytes: usize,
    transaction_active: bool,
}

/// Opens a `SQLite` database confined beneath a canonical directory root.
///
/// # Errors
///
/// Rejects malformed limits, non-canonical roots, absolute/traversing paths,
/// symlink escapes, `SQLite` setup failures, and exhausted handle capacity.
pub fn open_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let config = match arguments {
        [Value::Record(config)] => config,
        [_] => return Err(invalid("open config must be a record")),
        _ => return Err(invalid("open expects exactly one argument")),
    };
    exact_fields(
        config,
        &[
            "root",
            "path",
            "busy_timeout_ms",
            "max_rows",
            "max_result_bytes",
        ],
        "open config",
    )?;
    let root = PathBuf::from(string_field(config, "root")?);
    if !root.is_absolute() {
        return Err(invalid("root must be an absolute directory"));
    }
    let root = root
        .canonicalize()
        .map_err(|_| invalid("root must be an existing directory"))?;
    if !root.is_dir() {
        return Err(invalid("root must be an existing directory"));
    }
    let relative = confined_relative_path(string_field(config, "path")?)?;
    let target = root.join(relative);
    verify_target(&root, &target)?;
    let busy_timeout_ms = bounded_i64(config, "busy_timeout_ms", 0, MAX_BUSY_TIMEOUT_MS)?;
    let max_rows = bounded_i64(config, "max_rows", 1, MAX_ROWS)?;
    let max_result_bytes = bounded_i64(config, "max_result_bytes", 1, MAX_RESULT_BYTES)?;
    let connection = Connection::open_with_flags(
        target,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sqlite_error("open"))?;
    connection
        .busy_timeout(Duration::from_millis(
            u64::try_from(busy_timeout_ms).map_err(|_| invalid("busy timeout is invalid"))?,
        ))
        .map_err(sqlite_error("busy-timeout setup"))?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA trusted_schema = OFF;")
        .map_err(sqlite_error("security setup"))?;
    let database = Database {
        connection,
        max_rows: usize::try_from(max_rows).map_err(|_| invalid("max_rows is invalid"))?,
        max_result_bytes: usize::try_from(max_result_bytes)
            .map_err(|_| invalid("max_result_bytes is invalid"))?,
        transaction_active: false,
    };
    CONNECTIONS.with_borrow_mut(|registry| {
        registry
            .insert(CONNECTION_TYPE, database)
            .map(Value::Handle)
            .map_err(handle_error)
    })
}

/// Executes one parameterized statement without returning rows.
///
/// # Errors
///
/// Rejects invalid handles, SQL/parameter violations, and `SQLite` failures.
pub fn execute_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let (handle, sql, parameters) = statement_arguments(arguments, "execute")?;
    reject_transaction_sql(sql)?;
    let parameters = parse_parameters(parameters)?;
    CONNECTIONS.with_borrow_mut(|registry| {
        let database = registry
            .get_mut::<Database>(handle, CONNECTION_TYPE)
            .map_err(handle_error)?;
        let mut statement = database
            .connection
            .prepare(sql)
            .map_err(sqlite_error("prepare"))?;
        require_parameter_count(&statement, parameters.len())?;
        let rows = statement
            .execute(params_from_iter(parameters.iter()))
            .map_err(sqlite_error("execute"))?;
        let rows = i64::try_from(rows).map_err(|_| extension("affected-row count overflow"))?;
        Ok(record([
            ("rows_affected", Value::I64(rows)),
            (
                "last_insert_rowid",
                Value::I64(database.connection.last_insert_rowid()),
            ),
        ]))
    })
}

/// Runs one parameterized query and materializes a bounded result record.
///
/// # Errors
///
/// Rejects invalid handles, SQL/parameter violations, row/byte budget excess,
/// unsupported `SQLite` value kinds, and `SQLite` failures.
pub fn query_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let (handle, sql, parameters) = statement_arguments(arguments, "query")?;
    reject_transaction_sql(sql)?;
    let parameters = parse_parameters(parameters)?;
    CONNECTIONS.with_borrow_mut(|registry| {
        let database = registry
            .get_mut::<Database>(handle, CONNECTION_TYPE)
            .map_err(handle_error)?;
        query(database, sql, &parameters)
    })
}

/// Begins one explicit deferred transaction.
///
/// # Errors
///
/// Rejects invalid handles, nested transactions, and `SQLite` failures.
pub fn begin_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    transaction_command(arguments, "begin", false, "BEGIN", true)
}

/// Commits the active explicit transaction.
///
/// # Errors
///
/// Rejects invalid handles, missing transactions, and `SQLite` failures.
pub fn commit_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    transaction_command(arguments, "commit", true, "COMMIT", false)
}

/// Rolls back the active explicit transaction.
///
/// # Errors
///
/// Rejects invalid handles, missing transactions, and `SQLite` failures.
pub fn rollback_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    transaction_command(arguments, "rollback", true, "ROLLBACK", false)
}

/// Rolls back an active transaction, closes the database, and stales its handle.
///
/// # Errors
///
/// Rejects malformed/stale handles or a rollback failure.
pub fn close_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments, "close")?;
    CONNECTIONS.with_borrow_mut(|registry| {
        let database = registry
            .get_mut::<Database>(handle, CONNECTION_TYPE)
            .map_err(handle_error)?;
        if database.transaction_active {
            database
                .connection
                .execute_batch("ROLLBACK")
                .map_err(sqlite_error("rollback during close"))?;
            database.transaction_active = false;
        }
        registry
            .remove::<Database>(handle, CONNECTION_TYPE)
            .map_err(handle_error)?;
        Ok(Value::Bool(true))
    })
}

fn query(database: &mut Database, sql: &str, parameters: &[SqlValue]) -> Result<Value, OwnedError> {
    let mut statement = database
        .connection
        .prepare(sql)
        .map_err(sqlite_error("prepare"))?;
    require_parameter_count(&statement, parameters.len())?;
    let column_count = statement.column_count();
    if column_count > MAX_COLUMNS {
        return Err(extension("query column count exceeds 256"));
    }
    let names = (0..column_count)
        .map(|index| {
            statement
                .column_name(index)
                .map(str::to_owned)
                .map_err(sqlite_error("column metadata"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut used_bytes = names.iter().map(String::len).sum::<usize>();
    enforce_result_bytes(used_bytes, database.max_result_bytes)?;
    let mut rows = statement
        .query(params_from_iter(parameters.iter()))
        .map_err(sqlite_error("query"))?;
    let mut output_rows = Vec::new();
    while let Some(row) = rows.next().map_err(sqlite_error("row read"))? {
        if output_rows.len() >= database.max_rows {
            return Err(extension(format!(
                "query exceeds max_rows ({})",
                database.max_rows
            )));
        }
        let mut fields = BTreeMap::from([(
            "count".to_owned(),
            Value::I64(i64::try_from(column_count).map_err(|_| extension("column overflow"))?),
        )]);
        for index in 0..column_count {
            let value = output_value(row.get_ref(index).map_err(sqlite_error("column read"))?)?;
            used_bytes = used_bytes.saturating_add(value_size(&value));
            enforce_result_bytes(used_bytes, database.max_result_bytes)?;
            fields.insert(format!("item_{index}"), value);
        }
        output_rows.push(Value::Record(fields));
    }
    let mut result = BTreeMap::from([
        (
            "count".to_owned(),
            Value::I64(i64::try_from(output_rows.len()).map_err(|_| extension("row overflow"))?),
        ),
        ("columns".to_owned(), indexed_strings(&names)?),
    ]);
    for (index, row) in output_rows.into_iter().enumerate() {
        result.insert(format!("row_{index}"), row);
    }
    Ok(Value::Record(result))
}

fn transaction_command(
    arguments: &[Value],
    name: &str,
    required_state: bool,
    sql: &str,
    next_state: bool,
) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments, name)?;
    CONNECTIONS.with_borrow_mut(|registry| {
        let database = registry
            .get_mut::<Database>(handle, CONNECTION_TYPE)
            .map_err(handle_error)?;
        if database.transaction_active != required_state {
            return Err(invalid(if required_state {
                "no transaction is active"
            } else {
                "a transaction is already active"
            }));
        }
        database
            .connection
            .execute_batch(sql)
            .map_err(sqlite_error(name))?;
        database.transaction_active = next_state;
        Ok(Value::Bool(true))
    })
}

fn statement_arguments<'a>(
    arguments: &'a [Value],
    name: &str,
) -> Result<(NativeHandle, &'a str, &'a Value), OwnedError> {
    match arguments {
        [Value::Handle(handle), Value::String(sql), parameters] => {
            if sql.is_empty() || sql.len() > MAX_SQL_BYTES {
                return Err(invalid("SQL must contain 1..65536 UTF-8 bytes"));
            }
            Ok((*handle, sql, parameters))
        }
        [_, _, _] => Err(invalid(format!(
            "{name} expects a connection handle, SQL string, and parameter record"
        ))),
        _ => Err(invalid(format!("{name} expects exactly three arguments"))),
    }
}

fn parse_parameters(value: &Value) -> Result<Vec<SqlValue>, OwnedError> {
    let Value::Record(fields) = value else {
        return Err(invalid("parameters must be an indexed record"));
    };
    let count = usize::try_from(bounded_i64(
        fields,
        "count",
        0,
        i64::try_from(MAX_PARAMETERS).map_err(|_| invalid("parameter limit overflow"))?,
    )?)
    .map_err(|_| invalid("parameter count is invalid"))?;
    if fields.len() != count + 1 {
        return Err(invalid("parameters must contain count and item_0..item_N"));
    }
    (0..count)
        .map(|index| {
            fields
                .get(&format!("item_{index}"))
                .ok_or_else(|| invalid(format!("missing parameter item_{index}")))
                .and_then(parameter_value)
        })
        .collect()
}

fn parameter_value(value: &Value) -> Result<SqlValue, OwnedError> {
    match value {
        Value::Nil => Ok(SqlValue::Null),
        Value::Bool(value) => Ok(SqlValue::Integer(i64::from(*value))),
        Value::I64(value) => Ok(SqlValue::Integer(*value)),
        Value::F64(value) if value.is_finite() => Ok(SqlValue::Real(*value)),
        Value::F64(_) => Err(invalid("floating parameters must be finite")),
        Value::String(value) => Ok(SqlValue::Text(value.clone())),
        Value::Bytes(value) => Ok(SqlValue::Blob(value.clone())),
        Value::Array(_) | Value::Handle(_) | Value::Record(_) => Err(invalid(
            "parameters support only nil, bool, i64, finite f64, string, and bytes",
        )),
    }
}

fn output_value(value: ValueRef<'_>) -> Result<Value, OwnedError> {
    Ok(match value {
        ValueRef::Null => Value::Nil,
        ValueRef::Integer(value) => Value::I64(value),
        ValueRef::Real(value) => Value::F64(value),
        ValueRef::Text(value) => Value::String(
            std::str::from_utf8(value)
                .map_err(|_| extension("SQLite returned invalid UTF-8 text"))?
                .to_owned(),
        ),
        ValueRef::Blob(value) => Value::Bytes(value.to_vec()),
    })
}

fn confined_relative_path(value: &str) -> Result<&Path, OwnedError> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(invalid("database path must be a confined relative path"));
    }
    Ok(path)
}

fn verify_target(root: &Path, target: &Path) -> Result<(), OwnedError> {
    let parent = target
        .parent()
        .ok_or_else(|| invalid("database path has no parent"))?;
    let parent = parent
        .canonicalize()
        .map_err(|_| invalid("database parent directory must already exist"))?;
    if !parent.starts_with(root) {
        return Err(invalid("database path escapes the configured root"));
    }
    if target.exists() {
        let canonical = target
            .canonicalize()
            .map_err(|_| invalid("database path cannot be resolved"))?;
        if !canonical.starts_with(root) {
            return Err(invalid("database path escapes the configured root"));
        }
    }
    Ok(())
}

fn reject_transaction_sql(sql: &str) -> Result<(), OwnedError> {
    let first = sql
        .trim_start()
        .split(|character: char| character.is_ascii_whitespace() || character == ';')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(
        first.as_str(),
        "BEGIN" | "COMMIT" | "END" | "ROLLBACK" | "SAVEPOINT" | "RELEASE"
    ) {
        Err(invalid(
            "transaction SQL must use the explicit begin/commit/rollback API",
        ))
    } else {
        Ok(())
    }
}

fn require_parameter_count(
    statement: &rusqlite::Statement<'_>,
    actual: usize,
) -> Result<(), OwnedError> {
    let expected = statement.parameter_count();
    if expected == actual {
        Ok(())
    } else {
        Err(invalid(format!(
            "SQL expects {expected} parameters but received {actual}"
        )))
    }
}

fn indexed_strings(values: &[String]) -> Result<Value, OwnedError> {
    let mut fields = BTreeMap::from([(
        "count".to_owned(),
        Value::I64(i64::try_from(values.len()).map_err(|_| extension("column overflow"))?),
    )]);
    for (index, value) in values.iter().enumerate() {
        fields.insert(format!("item_{index}"), Value::String(value.clone()));
    }
    Ok(Value::Record(fields))
}

fn value_size(value: &Value) -> usize {
    match value {
        Value::Bool(_) => 1,
        Value::I64(_) | Value::F64(_) => 8,
        Value::String(value) => value.len(),
        Value::Bytes(value) => value.len(),
        Value::Nil | Value::Array(_) | Value::Handle(_) | Value::Record(_) => 0,
    }
}

fn enforce_result_bytes(actual: usize, maximum: usize) -> Result<(), OwnedError> {
    if actual > maximum {
        Err(extension(format!(
            "query exceeds max_result_bytes ({maximum})"
        )))
    } else {
        Ok(())
    }
}

fn handle_argument(arguments: &[Value], name: &str) -> Result<NativeHandle, OwnedError> {
    match arguments {
        [Value::Handle(handle)] => Ok(*handle),
        [_] => Err(invalid(format!("{name} expects a connection handle"))),
        _ => Err(invalid(format!("{name} expects exactly one argument"))),
    }
}

fn exact_fields(
    fields: &BTreeMap<String, Value>,
    expected: &[&str],
    context: &str,
) -> Result<(), OwnedError> {
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    let actual = fields.keys().map(String::as_str).collect::<BTreeSet<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(invalid(format!(
            "{context} fields do not match the V1 contract"
        )))
    }
}

fn string_field<'a>(
    fields: &'a BTreeMap<String, Value>,
    name: &str,
) -> Result<&'a str, OwnedError> {
    match fields.get(name) {
        Some(Value::String(value)) => Ok(value),
        Some(_) => Err(invalid(format!("{name} must be a string"))),
        None => Err(invalid(format!("missing field: {name}"))),
    }
}

fn bounded_i64(
    fields: &BTreeMap<String, Value>,
    name: &str,
    minimum: i64,
    maximum: i64,
) -> Result<i64, OwnedError> {
    match fields.get(name) {
        Some(Value::I64(value)) if (minimum..=maximum).contains(value) => Ok(*value),
        Some(Value::I64(_)) => Err(invalid(format!(
            "{name} must be between {minimum} and {maximum}"
        ))),
        Some(_) => Err(invalid(format!("{name} must be i64"))),
        None => Err(invalid(format!("missing field: {name}"))),
    }
}

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn sqlite_error(context: &str) -> impl FnOnce(rusqlite::Error) -> OwnedError + '_ {
    move |error| extension(format!("SQLite {context} failed: {error}"))
}

fn handle_error(error: HandleError) -> OwnedError {
    invalid(match error {
        HandleError::Inactive => "SQLite registry is inactive",
        HandleError::Exhausted => "SQLite connection capacity is exhausted",
        HandleError::WrongExtension => "SQLite handle belongs to another extension",
        HandleError::WrongType => "SQLite handle has the wrong resource type",
        HandleError::Stale => "SQLite handle is stale",
    })
}

fn invalid(message: impl Into<String>) -> OwnedError {
    OwnedError::invalid_argument(message)
}

fn extension(message: impl Into<String>) -> OwnedError {
    OwnedError::extension(message)
}
