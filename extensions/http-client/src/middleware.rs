use std::collections::BTreeMap;

use mlpl_extension_sdk::{OwnedError, Value};
use mlpl_http_contract::{MIDDLEWARE_ORDER, MiddlewarePolicy};

/// Validates middleware TOML and returns its normalized execution plan.
///
/// # Errors
///
/// Returns an invalid-argument error for malformed, unknown, disallowed, or
/// unsupported policy fields.
pub fn middleware_plan_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let source = match arguments {
        [Value::String(source)] => source,
        [_] => {
            return Err(OwnedError::invalid_argument(
                "middleware config must be TOML text",
            ));
        }
        _ => {
            return Err(OwnedError::invalid_argument(
                "middleware_plan expects exactly one argument",
            ));
        }
    };
    let policy = MiddlewarePolicy::parse(source)
        .map_err(|error| OwnedError::invalid_argument(error.message()))?;
    let cors_origin_count = i64::try_from(policy.allowed_origins().len())
        .map_err(|_| OwnedError::invalid_argument("CORS origin count overflow"))?;
    Ok(Value::Record(BTreeMap::from([
        ("version".into(), Value::I64(i64::from(policy.version()))),
        ("order".into(), Value::String(MIDDLEWARE_ORDER.into())),
        (
            "authorization_owner".into(),
            Value::String(policy.authorization_owner().into()),
        ),
        ("cors_origin_count".into(), Value::I64(cors_origin_count)),
        (
            "token_source".into(),
            Value::String(policy.token_source().into()),
        ),
        (
            "token_verification".into(),
            Value::String(policy.token_verification().into()),
        ),
    ])))
}
