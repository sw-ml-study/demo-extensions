use std::collections::BTreeMap;

use mlpl_extension_sdk::Value;
use mlpl_extension_sqlite::{
    begin_value, close_value, commit_value, execute_value, open_value, query_value, rollback_value,
};

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn params(values: impl IntoIterator<Item = Value>) -> Value {
    let values = values.into_iter().collect::<Vec<_>>();
    let mut fields = BTreeMap::from([(
        "count".to_owned(),
        Value::I64(i64::try_from(values.len()).unwrap()),
    )]);
    for (index, value) in values.into_iter().enumerate() {
        fields.insert(format!("item_{index}"), value);
    }
    Value::Record(fields)
}

fn config(root: &std::path::Path, path: &str, max_rows: i64, max_result_bytes: i64) -> Value {
    record([
        ("root", Value::String(root.display().to_string())),
        ("path", Value::String(path.to_owned())),
        ("busy_timeout_ms", Value::I64(1_000)),
        ("max_rows", Value::I64(max_rows)),
        ("max_result_bytes", Value::I64(max_result_bytes)),
    ])
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Record(fields) = value else {
        panic!("expected record")
    };
    fields.get(name).unwrap()
}

#[test]
fn parameterized_execute_query_transactions_and_cleanup_are_deterministic() {
    let temporary = tempfile::tempdir().unwrap();
    let connection = open_value(&[config(temporary.path(), "runs.sqlite", 10, 4096)]).unwrap();
    execute_value(&[
        connection.clone(),
        Value::String("CREATE TABLE runs (name TEXT NOT NULL, score REAL NOT NULL)".into()),
        params([]),
    ])
    .unwrap();
    begin_value(std::slice::from_ref(&connection)).unwrap();
    execute_value(&[
        connection.clone(),
        Value::String("INSERT INTO runs(name, score) VALUES (?1, ?2)".into()),
        params([Value::String("first".into()), Value::F64(0.75)]),
    ])
    .unwrap();
    rollback_value(std::slice::from_ref(&connection)).unwrap();
    begin_value(std::slice::from_ref(&connection)).unwrap();
    execute_value(&[
        connection.clone(),
        Value::String("INSERT INTO runs(name, score) VALUES (?1, ?2)".into()),
        params([Value::String("kept".into()), Value::F64(0.875)]),
    ])
    .unwrap();
    commit_value(std::slice::from_ref(&connection)).unwrap();

    let rows = query_value(&[
        connection.clone(),
        Value::String("SELECT name, score FROM runs WHERE score > ?1 ORDER BY score".into()),
        params([Value::F64(0.5)]),
    ])
    .unwrap();
    assert_eq!(field(&rows, "count"), &Value::I64(1));
    let row = field(&rows, "row_0");
    assert_eq!(field(row, "item_0"), &Value::String("kept".into()));
    assert_eq!(field(row, "item_1"), &Value::F64(0.875));

    close_value(std::slice::from_ref(&connection)).unwrap();
    assert!(query_value(&[connection, Value::String("SELECT 1".into()), params([])]).is_err());
}

#[test]
fn confinement_parameters_and_result_budgets_fail_closed() {
    let temporary = tempfile::tempdir().unwrap();
    assert!(open_value(&[config(temporary.path(), "../escape.sqlite", 10, 4096)]).is_err());
    let connection = open_value(&[config(temporary.path(), "bounded.sqlite", 1, 16)]).unwrap();
    execute_value(&[
        connection.clone(),
        Value::String("CREATE TABLE values_ (value TEXT)".into()),
        params([]),
    ])
    .unwrap();
    assert!(
        execute_value(&[
            connection.clone(),
            Value::String("INSERT INTO values_ VALUES (?1)".into()),
            params([]),
        ])
        .is_err()
    );
    execute_value(&[
        connection.clone(),
        Value::String("INSERT INTO values_ VALUES (?1)".into()),
        params([Value::String("a result larger than sixteen bytes".into())]),
    ])
    .unwrap();
    assert!(
        query_value(&[
            connection,
            Value::String("SELECT value FROM values_".into()),
            params([])
        ])
        .is_err()
    );
}

#[test]
fn row_limit_and_close_rollback_fail_closed() {
    let temporary = tempfile::tempdir().unwrap();
    let configuration = config(temporary.path(), "state.sqlite", 1, 4096);
    let connection = open_value(std::slice::from_ref(&configuration)).unwrap();
    execute_value(&[
        connection.clone(),
        Value::String("CREATE TABLE items (value INTEGER)".into()),
        params([]),
    ])
    .unwrap();
    execute_value(&[
        connection.clone(),
        Value::String("INSERT INTO items VALUES (1), (2)".into()),
        params([]),
    ])
    .unwrap();
    assert!(
        query_value(&[
            connection.clone(),
            Value::String("SELECT value FROM items ORDER BY value".into()),
            params([]),
        ])
        .is_err()
    );
    begin_value(std::slice::from_ref(&connection)).unwrap();
    execute_value(&[
        connection.clone(),
        Value::String("INSERT INTO items VALUES (?1)".into()),
        params([Value::I64(3)]),
    ])
    .unwrap();
    close_value(std::slice::from_ref(&connection)).unwrap();

    let reopened = open_value(&[config(temporary.path(), "state.sqlite", 10, 4096)]).unwrap();
    let result = query_value(&[
        reopened,
        Value::String("SELECT count(*) FROM items WHERE value = 3".into()),
        params([]),
    ])
    .unwrap();
    assert_eq!(field(field(&result, "row_0"), "item_0"), &Value::I64(0));
}

#[cfg(unix)]
#[test]
fn existing_symlink_cannot_escape_the_root() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_database = outside.path().join("outside.sqlite");
    std::fs::write(&outside_database, []).unwrap();
    std::os::unix::fs::symlink(&outside_database, root.path().join("link.sqlite")).unwrap();
    assert!(open_value(&[config(root.path(), "link.sqlite", 10, 4096)]).is_err());
}
