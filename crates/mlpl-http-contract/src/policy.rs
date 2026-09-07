use std::fmt;

use serde::Deserialize;
use url::Url;

pub const MIDDLEWARE_ORDER: &str = "limits,cors_preflight,token_extraction,token_verification,mlpl_authorization,handler,response_headers";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MiddlewarePolicy {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PolicyError(&'static str);

impl MiddlewarePolicy {
    /// Parses and validates one V1 TOML middleware policy.
    ///
    /// # Errors
    ///
    /// Rejects malformed, unknown, out-of-budget, or unsupported fields.
    pub fn parse(source: &str) -> Result<Self, PolicyError> {
        let policy: Self =
            toml::from_str(source).map_err(|_| PolicyError("middleware config is malformed"))?;
        policy.validate()?;
        Ok(policy)
    }

    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    #[must_use]
    pub const fn max_header_bytes(&self) -> usize {
        self.limits.max_header_bytes
    }

    #[must_use]
    pub const fn max_body_bytes(&self) -> usize {
        self.limits.max_body_bytes
    }

    #[must_use]
    pub fn allowed_origins(&self) -> &[String] {
        &self.cors.allowed_origins
    }

    #[must_use]
    pub fn origin_allowed(&self, origin: &str) -> bool {
        self.cors.allowed_origins.iter().any(|item| item == origin)
    }

    #[must_use]
    pub fn method_allowed(&self, method: &str) -> bool {
        self.cors.allowed_methods.iter().any(|item| item == method)
    }

    #[must_use]
    pub fn allowed_methods(&self) -> &[String] {
        &self.cors.allowed_methods
    }

    #[must_use]
    pub fn token_source(&self) -> &str {
        &self.token.source
    }

    #[must_use]
    pub fn token_verification(&self) -> &str {
        &self.token.verification
    }

    #[must_use]
    pub fn authorization_owner(&self) -> &str {
        &self.authorization.owner
    }

    fn validate(&self) -> Result<(), PolicyError> {
        if self.version != 1 {
            return Err(PolicyError("middleware version must be 1"));
        }
        if self.limits.max_header_bytes == 0 || self.limits.max_header_bytes > 1024 * 1024 {
            return Err(PolicyError("max_header_bytes is outside the V1 bounds"));
        }
        if self.limits.max_body_bytes == 0 || self.limits.max_body_bytes > 16 * 1024 * 1024 {
            return Err(PolicyError("max_body_bytes is outside the V1 bounds"));
        }
        if self.cors.allowed_origins.is_empty() || self.cors.allowed_origins.len() > 64 {
            return Err(PolicyError("CORS must declare between 1 and 64 origins"));
        }
        for origin in &self.cors.allowed_origins {
            let parsed = Url::parse(origin)
                .map_err(|_| PolicyError("CORS origin must be an absolute URL"))?;
            if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
                return Err(PolicyError("CORS origin must use http or https"));
            }
            if parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
                return Err(PolicyError(
                    "CORS origin must not contain a path, query, or fragment",
                ));
            }
        }
        if self.cors.allowed_methods.is_empty()
            || self.cors.allowed_methods.len() > 16
            || self.cors.allowed_methods.iter().any(|method| {
                !matches!(
                    method.as_str(),
                    "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
                )
            })
        {
            return Err(PolicyError(
                "CORS allowed_methods contains an invalid method",
            ));
        }
        if self.token.source != "authorization_bearer" && self.token.source != "none" {
            return Err(PolicyError("token source is not supported"));
        }
        if self.token.verification != "none" {
            return Err(PolicyError(
                "token verification is reserved until a verifier extension is configured",
            ));
        }
        if self.authorization.owner != "mlpl" {
            return Err(PolicyError("authorization owner must be mlpl"));
        }
        Ok(())
    }
}

impl PolicyError {
    #[must_use]
    pub const fn message(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for PolicyError {}
