use std::collections::BTreeMap;

use mlpl_extension_sdk::{OwnedError, Value};
use serde::Deserialize;
use url::Url;

const ORDER: &str = "limits,cors_preflight,token_extraction,token_verification,mlpl_authorization,handler,response_headers";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MiddlewareConfig {
    version: u32,
    limits: Limits,
    cors: Cors,
    token: Token,
    authorization: Authorization,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Limits {
    max_header_bytes: usize,
    max_body_bytes: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cors {
    allowed_origins: Vec<String>,
    allowed_methods: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Token {
    source: String,
    verification: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authorization {
    owner: String,
}

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
    let config: MiddlewareConfig = toml::from_str(source)
        .map_err(|_| OwnedError::invalid_argument("middleware config is malformed"))?;
    validate(&config)?;
    let cors_origin_count = i64::try_from(config.cors.allowed_origins.len())
        .map_err(|_| OwnedError::invalid_argument("CORS origin count overflow"))?;
    Ok(Value::Record(BTreeMap::from([
        ("version".into(), Value::I64(i64::from(config.version))),
        ("order".into(), Value::String(ORDER.into())),
        (
            "authorization_owner".into(),
            Value::String(config.authorization.owner),
        ),
        ("cors_origin_count".into(), Value::I64(cors_origin_count)),
        ("token_source".into(), Value::String(config.token.source)),
        (
            "token_verification".into(),
            Value::String(config.token.verification),
        ),
    ])))
}

fn validate(config: &MiddlewareConfig) -> Result<(), OwnedError> {
    if config.version != 1 {
        return Err(invalid("middleware version must be 1"));
    }
    if config.limits.max_header_bytes == 0 || config.limits.max_header_bytes > 1024 * 1024 {
        return Err(invalid("max_header_bytes is outside the V1 bounds"));
    }
    if config.limits.max_body_bytes == 0 || config.limits.max_body_bytes > 16 * 1024 * 1024 {
        return Err(invalid("max_body_bytes is outside the V1 bounds"));
    }
    if config.cors.allowed_origins.is_empty() || config.cors.allowed_origins.len() > 64 {
        return Err(invalid("CORS must declare between 1 and 64 origins"));
    }
    for origin in &config.cors.allowed_origins {
        let parsed =
            Url::parse(origin).map_err(|_| invalid("CORS origin must be an absolute URL"))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(invalid("CORS origin must use http or https"));
        }
        if parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
            return Err(invalid(
                "CORS origin must not contain a path, query, or fragment",
            ));
        }
    }
    if config.cors.allowed_methods.is_empty()
        || config.cors.allowed_methods.len() > 16
        || config.cors.allowed_methods.iter().any(|method| {
            !matches!(
                method.as_str(),
                "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
            )
        })
    {
        return Err(invalid("CORS allowed_methods contains an invalid method"));
    }
    if config.token.source != "authorization_bearer" && config.token.source != "none" {
        return Err(invalid("token source is not supported"));
    }
    if config.token.verification != "none" {
        return Err(invalid(
            "token verification is reserved until a verifier extension is configured",
        ));
    }
    if config.authorization.owner != "mlpl" {
        return Err(invalid("authorization owner must be mlpl"));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> OwnedError {
    OwnedError::invalid_argument(message)
}
