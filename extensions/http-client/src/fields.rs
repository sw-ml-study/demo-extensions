//! Shared record-field validation helpers for the HTTP extension's contracts.

use std::collections::{BTreeMap, BTreeSet};

use mlpl_extension_sdk::{OwnedError, Value};

pub(crate) fn bounded_i64(
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
        None => Err(invalid(format!("missing request field: {name}"))),
    }
}

pub(crate) fn string_field<'a>(
    fields: &'a BTreeMap<String, Value>,
    name: &str,
) -> Result<&'a str, OwnedError> {
    match fields.get(name) {
        Some(Value::String(value)) => Ok(value),
        Some(_) => Err(invalid(format!("{name} must be a string"))),
        None => Err(invalid(format!("missing field: {name}"))),
    }
}

pub(crate) fn require_exact_fields(
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

pub(crate) fn single_record<'a>(
    arguments: &'a [Value],
    context: &str,
) -> Result<&'a BTreeMap<String, Value>, OwnedError> {
    match arguments {
        [Value::Record(fields)] => Ok(fields),
        [_] => Err(invalid(format!("{context} must be a record"))),
        _ => Err(invalid(format!("{context} expects exactly one argument"))),
    }
}

pub(crate) fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

pub(crate) fn invalid(message: impl Into<String>) -> OwnedError {
    OwnedError::invalid_argument(message)
}
