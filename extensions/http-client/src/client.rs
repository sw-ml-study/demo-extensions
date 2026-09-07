use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::time::Duration;

use mlpl_extension_sdk::{OwnedError, Value};
use url::Url;

const MAX_TIMEOUT_MS: i64 = 120_000;
const MAX_RESPONSE_BYTES: i64 = 16 * 1024 * 1024;
const MAX_REQUEST_BYTES: usize = 16 * 1024 * 1024;
const MAX_HEADER_COUNT: usize = 128;
const MAX_HEADER_BYTES: usize = 64 * 1024;

struct HttpRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    timeout: Duration,
    max_response_bytes: usize,
    max_redirects: u32,
}

/// Performs one validated, bounded synchronous HTTP request.
///
/// # Errors
///
/// Returns an invalid-argument error for malformed request records and an
/// extension error for transport or bounded-response failures.
pub fn request_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let request = HttpRequest::parse(arguments)?;
    execute(&request)
}

impl HttpRequest {
    fn parse(arguments: &[Value]) -> Result<Self, OwnedError> {
        let fields = match arguments {
            [Value::Record(fields)] => fields,
            [_] => return Err(invalid("request must be a record")),
            _ => return Err(invalid("request expects exactly one argument")),
        };
        require_exact_fields(
            fields,
            &[
                "method",
                "url",
                "headers",
                "body",
                "timeout_ms",
                "max_response_bytes",
                "max_redirects",
            ],
            "request",
        )?;
        let method = string_field(fields, "method")?.to_ascii_uppercase();
        if !matches!(
            method.as_str(),
            "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
        ) {
            return Err(invalid("method is not supported"));
        }
        let url = string_field(fields, "url")?.to_owned();
        let parsed = Url::parse(&url).map_err(|_| invalid("url must be absolute"))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(invalid("url scheme must be http or https"));
        }
        if parsed.host_str().is_none() {
            return Err(invalid("url must contain a host"));
        }
        let timeout_ms = bounded_i64(fields, "timeout_ms", 1, MAX_TIMEOUT_MS)?;
        let max_response_bytes = bounded_i64(fields, "max_response_bytes", 1, MAX_RESPONSE_BYTES)?;
        let max_redirects = bounded_i64(fields, "max_redirects", 0, 10)?;
        let body = match fields.get("body") {
            Some(Value::Bytes(body)) if body.len() <= MAX_REQUEST_BYTES => body.clone(),
            Some(Value::Bytes(_)) => return Err(invalid("body exceeds the request byte limit")),
            _ => return Err(invalid("body must be bytes")),
        };
        let headers = parse_headers(
            fields
                .get("headers")
                .ok_or_else(|| invalid("missing request field: headers"))?,
        )?;
        Ok(Self {
            method,
            url,
            headers,
            body,
            timeout: Duration::from_millis(
                u64::try_from(timeout_ms).map_err(|_| invalid("timeout_ms is invalid"))?,
            ),
            max_response_bytes: usize::try_from(max_response_bytes)
                .map_err(|_| invalid("max_response_bytes is invalid"))?,
            max_redirects: u32::try_from(max_redirects)
                .map_err(|_| invalid("max_redirects is invalid"))?,
        })
    }
}

fn execute(request: &HttpRequest) -> Result<Value, OwnedError> {
    let agent = ureq::AgentBuilder::new()
        .timeout(request.timeout)
        .redirects(request.max_redirects)
        .build();
    let mut outgoing = agent.request(&request.method, &request.url);
    for (name, value) in &request.headers {
        outgoing = outgoing.set(name, value);
    }
    let result = if request.body.is_empty() {
        outgoing.call()
    } else {
        outgoing.send_bytes(&request.body)
    };
    let response = match result {
        Ok(response) | Err(ureq::Error::Status(_, response)) => response,
        Err(ureq::Error::Transport(error)) => {
            return Err(OwnedError::extension(format!(
                "HTTP transport failure: {}",
                error.kind()
            )));
        }
    };
    encode_response(response, request.max_response_bytes)
}

fn encode_response(response: ureq::Response, max_body: usize) -> Result<Value, OwnedError> {
    let status = i64::from(response.status());
    let final_url = response.get_url().to_owned();
    let headers = response_headers(&response)?;
    let mut body = Vec::new();
    response
        .into_reader()
        .take((max_body as u64).saturating_add(1))
        .read_to_end(&mut body)
        .map_err(|_| OwnedError::extension("HTTP response body read failed"))?;
    if body.len() > max_body {
        return Err(OwnedError::extension(format!(
            "HTTP response body exceeds max_response_bytes ({max_body})"
        )));
    }
    Ok(record([
        ("status", Value::I64(status)),
        ("headers", indexed_headers(&headers)?),
        ("body", Value::Bytes(body)),
        ("final_url", Value::String(final_url)),
    ]))
}

fn response_headers(response: &ureq::Response) -> Result<Vec<(String, String)>, OwnedError> {
    let names = response
        .headers_names()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut headers = Vec::new();
    let mut bytes = 0_usize;
    for name in names {
        for value in response.all(&name) {
            bytes = bytes.saturating_add(name.len()).saturating_add(value.len());
            if headers.len() >= MAX_HEADER_COUNT || bytes > MAX_HEADER_BYTES {
                return Err(OwnedError::extension(
                    "HTTP response headers exceed configured safety limits",
                ));
            }
            headers.push((name.clone(), value.to_owned()));
        }
    }
    Ok(headers)
}

fn parse_headers(value: &Value) -> Result<Vec<(String, String)>, OwnedError> {
    let Value::Record(fields) = value else {
        return Err(invalid("headers must be an indexed record"));
    };
    let count = usize::try_from(bounded_i64(fields, "count", 0, 128)?)
        .map_err(|_| invalid("headers.count is invalid"))?;
    if fields.len() != count + 1 {
        return Err(invalid("headers must contain count and item_0..item_N"));
    }
    let mut headers = Vec::with_capacity(count);
    let mut bytes = 0_usize;
    for index in 0..count {
        let key = format!("item_{index}");
        let Some(Value::Record(header)) = fields.get(&key) else {
            return Err(invalid(format!("headers.{key} must be a record")));
        };
        require_exact_fields(header, &["name", "value"], &format!("headers.{key}"))?;
        let name = string_field(header, "name")?;
        let value = string_field(header, "value")?;
        if !valid_header_name(name) {
            return Err(invalid(format!("headers.{key}.name is invalid")));
        }
        if value.contains(['\r', '\n']) {
            return Err(invalid(format!("headers.{key}.value is invalid")));
        }
        bytes = bytes.saturating_add(name.len()).saturating_add(value.len());
        if bytes > MAX_HEADER_BYTES {
            return Err(invalid("request headers exceed the byte limit"));
        }
        headers.push((name.to_owned(), value.to_owned()));
    }
    Ok(headers)
}

fn indexed_headers(headers: &[(String, String)]) -> Result<Value, OwnedError> {
    let count = i64::try_from(headers.len())
        .map_err(|_| OwnedError::extension("HTTP response header count overflow"))?;
    let mut fields = BTreeMap::from([("count".to_owned(), Value::I64(count))]);
    for (index, (name, value)) in headers.iter().enumerate() {
        fields.insert(
            format!("item_{index}"),
            record([
                ("name", Value::String(name.clone())),
                ("value", Value::String(value.clone())),
            ]),
        );
    }
    Ok(Value::Record(fields))
}

fn valid_header_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
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
        None => Err(invalid(format!("missing request field: {name}"))),
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

fn require_exact_fields(
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

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn invalid(message: impl Into<String>) -> OwnedError {
    OwnedError::invalid_argument(message)
}
