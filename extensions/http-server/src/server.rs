use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use mlpl_extension_sdk::{HandleError, HandleRegistry, NativeHandle, OwnedError, Value};
use mlpl_http_contract::MiddlewarePolicy;

const EXTENSION_ID: u64 = 0x48_54_54_50_53_45_52_56;
const SERVER_TYPE: u64 = 1;
const MAX_SERVERS: usize = 16;
const MAX_PENDING: i64 = 64;
const MAX_POLL_MS: i64 = 120_000;
const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
const MAX_RESPONSE_HEADERS: usize = 128;
const MAX_RESPONSE_HEADER_BYTES: usize = 64 * 1024;

thread_local! {
    static SERVERS: RefCell<HandleRegistry> = const {
        RefCell::new(HandleRegistry::with_limits(EXTENSION_ID, MAX_SERVERS, u32::MAX))
    };
}

struct Server {
    listener: TcpListener,
    policy: MiddlewarePolicy,
    pending: BTreeMap<u64, PendingResponse>,
    next_request_id: u64,
    max_pending: usize,
}

struct PendingResponse {
    stream: TcpStream,
    cors_origin: Option<String>,
}

struct Request {
    method: String,
    path: String,
    query: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    origin: Option<String>,
    bearer_token: Option<String>,
}

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// Opens one loopback-only nonblocking HTTP listener.
///
/// # Errors
///
/// Rejects malformed configuration or middleware and reports bind/handle
/// failures without returning a partial server.
pub fn listen_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let (config, policy_source) = match arguments {
        [Value::Record(config), Value::String(policy)] => (config, policy),
        [_, _] => {
            return Err(invalid(
                "listen expects a config record and middleware TOML",
            ));
        }
        _ => return Err(invalid("listen expects exactly two arguments")),
    };
    exact_fields(config, &["address", "port", "max_pending"], "listen config")?;
    let address = string_field(config, "address")?;
    if address != "127.0.0.1" {
        return Err(invalid("V1 listeners are restricted to IPv4 loopback"));
    }
    let port = bounded_i64(config, "port", 0, i64::from(u16::MAX))?;
    let max_pending = bounded_i64(config, "max_pending", 1, MAX_PENDING)?;
    let policy =
        MiddlewarePolicy::parse(policy_source).map_err(|error| invalid(error.message()))?;
    let listener = TcpListener::bind(format!("{address}:{port}"))
        .map_err(|_| OwnedError::extension("HTTP listener bind failed"))?;
    listener
        .set_nonblocking(true)
        .map_err(|_| OwnedError::extension("HTTP listener nonblocking setup failed"))?;
    let server = Server {
        listener,
        policy,
        pending: BTreeMap::new(),
        next_request_id: 1,
        max_pending: usize::try_from(max_pending).map_err(|_| invalid("max_pending is invalid"))?,
    };
    SERVERS.with_borrow_mut(|registry| {
        registry
            .insert(SERVER_TYPE, server)
            .map(Value::Handle)
            .map_err(handle_error)
    })
}

/// Returns the bound socket address for diagnostics and port-zero discovery.
///
/// # Errors
///
/// Rejects malformed, foreign, wrong-type, or stale server handles.
pub fn local_address_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments, 1)?;
    SERVERS.with_borrow(|registry| {
        let server = registry
            .get::<Server>(handle, SERVER_TYPE)
            .map_err(handle_error)?;
        let address = server
            .listener
            .local_addr()
            .map_err(|_| OwnedError::extension("HTTP listener address unavailable"))?;
        Ok(Value::String(address.to_string()))
    })
}

/// Polls for one bounded request and returns nil on timeout.
///
/// # Errors
///
/// Rejects invalid handles/timeouts and malformed or over-budget requests.
pub fn next_request_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let (handle, timeout_ms) = match arguments {
        [Value::Handle(handle), Value::I64(timeout_ms)]
            if (0..=MAX_POLL_MS).contains(timeout_ms) =>
        {
            (*handle, *timeout_ms)
        }
        [Value::Handle(_), Value::I64(_)] => {
            return Err(invalid("poll timeout_ms must be between 0 and 120000"));
        }
        [_, _] => {
            return Err(invalid(
                "next_request expects a server handle and timeout_ms",
            ));
        }
        _ => return Err(invalid("next_request expects exactly two arguments")),
    };
    let timeout = Duration::from_millis(
        u64::try_from(timeout_ms).map_err(|_| invalid("poll timeout is invalid"))?,
    );
    SERVERS.with_borrow_mut(|registry| {
        let server = registry
            .get_mut::<Server>(handle, SERVER_TYPE)
            .map_err(handle_error)?;
        server.poll(timeout)
    })
}

/// Sends one bounded response and consumes its pending request.
///
/// # Errors
///
/// Rejects invalid handles, unknown request IDs, malformed response records,
/// and response write failures.
pub fn respond_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let (handle, request_id, response) = match arguments {
        [
            Value::Handle(handle),
            Value::I64(request_id),
            Value::Record(response),
        ] => (*handle, *request_id, response),
        [_, _, _] => {
            return Err(invalid(
                "respond expects a server handle, request ID, and response record",
            ));
        }
        _ => return Err(invalid("respond expects exactly three arguments")),
    };
    let request_id = u64::try_from(request_id).map_err(|_| invalid("request ID is invalid"))?;
    let response = Response::parse(response)?;
    SERVERS.with_borrow_mut(|registry| {
        let server = registry
            .get_mut::<Server>(handle, SERVER_TYPE)
            .map_err(handle_error)?;
        let pending = server
            .pending
            .remove(&request_id)
            .ok_or_else(|| invalid("request ID is not pending"))?;
        write_response(pending.stream, response, pending.cors_origin.as_deref())?;
        Ok(Value::Bool(true))
    })
}

/// Closes one server and invalidates its handle and pending requests.
///
/// # Errors
///
/// Rejects malformed, foreign, wrong-type, or stale handles.
pub fn close_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments, 1)?;
    SERVERS.with_borrow_mut(|registry| {
        registry
            .remove::<Server>(handle, SERVER_TYPE)
            .map_err(handle_error)?;
        Ok(Value::Bool(true))
    })
}

impl Server {
    fn poll(&mut self, timeout: Duration) -> Result<Value, OwnedError> {
        if self.pending.len() >= self.max_pending {
            return Err(OwnedError::extension(
                "HTTP pending-request capacity is exhausted",
            ));
        }
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| invalid("poll deadline overflow"))?;
        loop {
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_nonblocking(false)
                        .map_err(|_| OwnedError::extension("HTTP connection setup failed"))?;
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    stream
                        .set_read_timeout(Some(remaining.max(Duration::from_millis(1))))
                        .map_err(|_| OwnedError::extension("HTTP read-timeout setup failed"))?;
                    let request = read_request(&mut stream, &self.policy)?;
                    if self.handle_cors(&request, &mut stream)? {
                        if Instant::now() >= deadline {
                            return Ok(Value::Nil);
                        }
                        continue;
                    }
                    let id = self.next_id()?;
                    let result = request_value(id, &request)?;
                    self.pending.insert(
                        id,
                        PendingResponse {
                            stream,
                            cors_origin: request.origin,
                        },
                    );
                    return Ok(result);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Ok(Value::Nil);
                    }
                    thread::sleep(Duration::from_millis(1));
                }
                Err(_) => return Err(OwnedError::extension("HTTP listener accept failed")),
            }
        }
    }

    fn handle_cors(&self, request: &Request, stream: &mut TcpStream) -> Result<bool, OwnedError> {
        let Some(origin) = request.origin.as_deref() else {
            return Ok(false);
        };
        if !self.policy.origin_allowed(origin) {
            write_simple(stream, 403, "Forbidden", None, &[])?;
            return Ok(true);
        }
        if request.method == "OPTIONS" {
            let requested = header_value(&request.headers, "access-control-request-method");
            if requested.is_some_and(|method| self.policy.method_allowed(method)) {
                let methods = self.policy.allowed_methods().join(", ");
                write_simple(
                    stream,
                    204,
                    "No Content",
                    Some(origin),
                    &[("access-control-allow-methods", methods.as_str())],
                )?;
            } else {
                write_simple(stream, 403, "Forbidden", Some(origin), &[])?;
            }
            return Ok(true);
        }
        if !self.policy.method_allowed(&request.method) {
            write_simple(stream, 405, "Method Not Allowed", Some(origin), &[])?;
            return Ok(true);
        }
        Ok(false)
    }

    fn next_id(&mut self) -> Result<u64, OwnedError> {
        let id = self.next_request_id;
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .ok_or_else(|| OwnedError::extension("HTTP request ID space exhausted"))?;
        Ok(id)
    }
}

impl Response {
    fn parse(fields: &BTreeMap<String, Value>) -> Result<Self, OwnedError> {
        exact_fields(fields, &["status", "headers", "body"], "response")?;
        let status = bounded_i64(fields, "status", 200, 599)?;
        let headers = parse_indexed_headers(
            fields
                .get("headers")
                .ok_or_else(|| invalid("missing response field: headers"))?,
            MAX_RESPONSE_HEADERS,
            MAX_RESPONSE_HEADER_BYTES,
        )?;
        if headers.iter().any(|(name, _)| {
            matches!(
                name.to_ascii_lowercase().as_str(),
                "content-length" | "connection" | "access-control-allow-origin"
            )
        }) {
            return Err(invalid("response contains a host-managed header"));
        }
        let body = match fields.get("body") {
            Some(Value::Bytes(body)) if body.len() <= MAX_RESPONSE_BYTES => body.clone(),
            Some(Value::Bytes(_)) => return Err(invalid("response body exceeds the V1 limit")),
            _ => return Err(invalid("response body must be bytes")),
        };
        Ok(Self {
            status: u16::try_from(status).map_err(|_| invalid("status is invalid"))?,
            headers,
            body,
        })
    }
}

fn read_request(stream: &mut TcpStream, policy: &MiddlewarePolicy) -> Result<Request, OwnedError> {
    let mut received = Vec::new();
    let mut chunk = [0_u8; 2048];
    let header_end = loop {
        let size = stream
            .read(&mut chunk)
            .map_err(|_| OwnedError::extension("HTTP request read failed"))?;
        if size == 0 {
            return Err(invalid("HTTP request ended before its headers"));
        }
        received.extend_from_slice(&chunk[..size]);
        if let Some(index) = find_header_end(&received) {
            break index;
        }
        if received.len() > policy.max_header_bytes() {
            return Err(invalid("HTTP request headers exceed the configured limit"));
        }
    };
    if header_end > policy.max_header_bytes() {
        return Err(invalid("HTTP request headers exceed the configured limit"));
    }
    let head = std::str::from_utf8(&received[..header_end])
        .map_err(|_| invalid("HTTP request headers must be UTF-8"))?;
    let mut lines = head.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| invalid("HTTP request line is missing"))?;
    let parts = request_line.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 3 || !matches!(parts[2], "HTTP/1.0" | "HTTP/1.1") {
        return Err(invalid("HTTP request line is invalid"));
    }
    let method = parts[0].to_ascii_uppercase();
    if !matches!(
        method.as_str(),
        "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
    ) {
        return Err(invalid("HTTP method is not supported"));
    }
    let (path, query) = parse_target(parts[1])?;
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| invalid("HTTP header line is invalid"))?;
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim().to_owned();
        if !valid_header_name(&name) || value.contains(['\r', '\n']) {
            return Err(invalid("HTTP header is invalid"));
        }
        headers.push((name, value));
        if headers.len() > 128 {
            return Err(invalid("HTTP request has too many headers"));
        }
    }
    if header_value(&headers, "transfer-encoding").is_some() {
        return Err(invalid("transfer-encoded requests are not supported in V1"));
    }
    let content_length = match header_value(&headers, "content-length") {
        Some(value) => value
            .parse::<usize>()
            .map_err(|_| invalid("content-length is invalid"))?,
        None => 0,
    };
    if content_length > policy.max_body_bytes() {
        return Err(invalid("HTTP request body exceeds the configured limit"));
    }
    let body_start = header_end + 4;
    while received.len().saturating_sub(body_start) < content_length {
        let size = stream
            .read(&mut chunk)
            .map_err(|_| OwnedError::extension("HTTP request body read failed"))?;
        if size == 0 {
            return Err(invalid("HTTP request body is truncated"));
        }
        received.extend_from_slice(&chunk[..size]);
        if received.len().saturating_sub(body_start) > policy.max_body_bytes() {
            return Err(invalid("HTTP request body exceeds the configured limit"));
        }
    }
    let origin = header_value(&headers, "origin").map(str::to_owned);
    let bearer_token = if policy.token_source() == "authorization_bearer" {
        header_value(&headers, "authorization")
            .and_then(|value| value.strip_prefix("Bearer "))
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    } else {
        None
    };
    Ok(Request {
        method,
        path,
        query,
        headers,
        body: received[body_start..body_start + content_length].to_vec(),
        origin,
        bearer_token,
    })
}

fn request_value(id: u64, request: &Request) -> Result<Value, OwnedError> {
    Ok(record([
        (
            "id",
            Value::I64(
                i64::try_from(id)
                    .map_err(|_| OwnedError::extension("HTTP request ID exceeds MLPL i64"))?,
            ),
        ),
        ("method", Value::String(request.method.clone())),
        ("path", Value::String(request.path.clone())),
        ("query", Value::String(request.query.clone())),
        ("headers", indexed_headers(&request.headers)?),
        ("body", Value::Bytes(request.body.clone())),
        (
            "origin",
            request.origin.clone().map_or(Value::Nil, Value::String),
        ),
        (
            "bearer_token",
            request
                .bearer_token
                .clone()
                .map_or(Value::Nil, Value::String),
        ),
        ("token_verified", Value::Bool(false)),
        ("authorization_owner", Value::String("mlpl".into())),
    ]))
}

fn write_response(
    mut stream: TcpStream,
    response: Response,
    cors_origin: Option<&str>,
) -> Result<(), OwnedError> {
    let reason = reason_phrase(response.status);
    let mut head = format!(
        "HTTP/1.1 {} {}\r\ncontent-length: {}\r\nconnection: close\r\n",
        response.status,
        reason,
        response.body.len()
    );
    if let Some(origin) = cors_origin {
        let _ = write!(head, "access-control-allow-origin: {origin}\r\n");
        head.push_str("vary: origin\r\n");
    }
    for (name, value) in response.headers {
        head.push_str(&name);
        head.push_str(": ");
        head.push_str(&value);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.write_all(&response.body))
        .and_then(|()| stream.flush())
        .map_err(|_| OwnedError::extension("HTTP response write failed"))
}

fn write_simple(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    cors_origin: Option<&str>,
    extra: &[(&str, &str)],
) -> Result<(), OwnedError> {
    let mut head =
        format!("HTTP/1.1 {status} {reason}\r\ncontent-length: 0\r\nconnection: close\r\n");
    if let Some(origin) = cors_origin {
        let _ = write!(head, "access-control-allow-origin: {origin}\r\n");
        head.push_str("vary: origin\r\n");
    }
    for (name, value) in extra {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.flush())
        .map_err(|_| OwnedError::extension("HTTP middleware response write failed"))
}

fn parse_indexed_headers(
    value: &Value,
    max_count: usize,
    max_bytes: usize,
) -> Result<Vec<(String, String)>, OwnedError> {
    let Value::Record(fields) = value else {
        return Err(invalid("headers must be an indexed record"));
    };
    let count_limit = i64::try_from(max_count).map_err(|_| invalid("header limit overflow"))?;
    let count = usize::try_from(bounded_i64(fields, "count", 0, count_limit)?)
        .map_err(|_| invalid("header count is invalid"))?;
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
        exact_fields(header, &["name", "value"], &format!("headers.{key}"))?;
        let name = string_field(header, "name")?.to_ascii_lowercase();
        let value = string_field(header, "value")?.to_owned();
        if !valid_header_name(&name) || value.contains(['\r', '\n']) {
            return Err(invalid(format!("headers.{key} is invalid")));
        }
        bytes = bytes.saturating_add(name.len()).saturating_add(value.len());
        if bytes > max_bytes {
            return Err(invalid("headers exceed the byte limit"));
        }
        headers.push((name, value));
    }
    Ok(headers)
}

fn indexed_headers(headers: &[(String, String)]) -> Result<Value, OwnedError> {
    let count = i64::try_from(headers.len())
        .map_err(|_| OwnedError::extension("HTTP header count overflow"))?;
    let mut fields = BTreeMap::from([("count".into(), Value::I64(count))]);
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

fn parse_target(target: &str) -> Result<(String, String), OwnedError> {
    if !target.starts_with('/') || target.contains(['\r', '\n']) {
        return Err(invalid("HTTP request target must use origin form"));
    }
    Ok(target.split_once('?').map_or_else(
        || (target.to_owned(), String::new()),
        |(path, query)| (path.to_owned(), query.to_owned()),
    ))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find_map(|(candidate, value)| (candidate == name).then_some(value.as_str()))
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

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        422 => "Unprocessable Content",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Response",
    }
}

fn handle_argument(arguments: &[Value], expected: usize) -> Result<NativeHandle, OwnedError> {
    if arguments.len() != expected {
        return Err(invalid(format!("expected {expected} argument(s)")));
    }
    match arguments.first() {
        Some(Value::Handle(handle)) => Ok(*handle),
        _ => Err(invalid("server must be a native handle")),
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

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn handle_error(error: HandleError) -> OwnedError {
    OwnedError::extension(format!("invalid HTTP server handle: {error:?}"))
}

fn invalid(message: impl Into<String>) -> OwnedError {
    OwnedError::invalid_argument(message)
}
