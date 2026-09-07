use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use mlpl_extension_http_client::{middleware_plan_value, request_value};
use mlpl_extension_sdk::Value;

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn headers(entries: &[(&str, &str)]) -> Value {
    let mut fields = BTreeMap::from([(
        "count".to_owned(),
        Value::I64(i64::try_from(entries.len()).unwrap()),
    )]);
    for (index, (name, value)) in entries.iter().enumerate() {
        fields.insert(
            format!("item_{index}"),
            record([
                ("name", Value::String((*name).to_owned())),
                ("value", Value::String((*value).to_owned())),
            ]),
        );
    }
    Value::Record(fields)
}

fn request(url: String, max_response_bytes: i64) -> Value {
    record([
        ("method", Value::String("POST".into())),
        ("url", Value::String(url)),
        ("headers", headers(&[("x-demo", "bounded")])),
        ("body", Value::Bytes(b"ping".to_vec())),
        ("timeout_ms", Value::I64(2_000)),
        ("max_response_bytes", Value::I64(max_response_bytes)),
        ("max_redirects", Value::I64(0)),
    ])
}

fn read_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut request = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let size = stream.read(&mut chunk).unwrap();
        request.extend_from_slice(&chunk[..size]);
        if size == 0 || request.windows(8).any(|window| window == b"\r\n\r\nping") {
            return request;
        }
    }
}

#[test]
fn loopback_request_preserves_status_headers_and_bytes() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let input = read_request(&mut stream);
        let request = String::from_utf8_lossy(&input);
        assert!(request.starts_with("POST /probe HTTP/1.1"));
        assert!(request.to_ascii_lowercase().contains("x-demo: bounded"));
        assert!(request.ends_with("ping"));
        stream
            .write_all(
                b"HTTP/1.1 418 Teapot\r\nContent-Length: 4\r\nX-Probe: yes\r\nConnection: close\r\n\r\npong",
            )
            .unwrap();
    });

    let response = request_value(&[request(format!("http://{address}/probe"), 16)]).unwrap();
    let Value::Record(fields) = response else {
        panic!("response must be a record");
    };
    assert_eq!(fields.get("status"), Some(&Value::I64(418)));
    assert_eq!(fields.get("body"), Some(&Value::Bytes(b"pong".to_vec())));
    assert_eq!(
        fields.get("final_url"),
        Some(&Value::String(format!("http://{address}/probe")))
    );
    server.join().unwrap();
}

#[test]
fn request_rejects_unsafe_or_unbounded_inputs_before_io() {
    let invalid_scheme = request("file:///etc/passwd".into(), 16);
    assert!(
        request_value(&[invalid_scheme])
            .unwrap_err()
            .message()
            .contains("http or https")
    );

    let zero_limit = request("http://127.0.0.1/".into(), 0);
    assert!(
        request_value(&[zero_limit])
            .unwrap_err()
            .message()
            .contains("max_response_bytes")
    );

    let mut injected = request("http://127.0.0.1/".into(), 16);
    let Value::Record(fields) = &mut injected else {
        unreachable!();
    };
    fields.insert(
        "headers".into(),
        headers(&[("x-safe", "yes\r\nx-injected: no")]),
    );
    assert!(
        request_value(&[injected])
            .unwrap_err()
            .message()
            .contains("value is invalid")
    );
}

#[test]
fn response_body_limit_fails_closed() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = read_request(&mut stream);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\ntoo-long")
            .unwrap();
    });

    let error = request_value(&[request(format!("http://{address}/"), 4)]).unwrap_err();
    assert!(error.message().contains("response body exceeds"));
    server.join().unwrap();
}

#[test]
fn middleware_config_has_fixed_security_order_and_mlpl_authorization() {
    let config = include_str!("../middleware.example.toml");
    let plan = middleware_plan_value(&[Value::String(config.into())]).unwrap();
    let Value::Record(fields) = plan else {
        panic!("middleware plan must be a record");
    };
    assert_eq!(fields.get("version"), Some(&Value::I64(1)));
    assert_eq!(
        fields.get("order"),
        Some(&Value::String(
            "limits,cors_preflight,token_extraction,token_verification,mlpl_authorization,handler,response_headers".into()
        ))
    );
    assert_eq!(
        fields.get("authorization_owner"),
        Some(&Value::String("mlpl".into()))
    );
}

#[test]
fn middleware_config_rejects_reordering_and_native_authorization() {
    let native_authorization = r#"
version = 1
[limits]
max_header_bytes = 1024
max_body_bytes = 1024
[cors]
allowed_origins = ["https://example.test"]
allowed_methods = ["GET"]
[token]
source = "none"
verification = "none"
[authorization]
owner = "rust"
"#;
    assert!(
        middleware_plan_value(&[Value::String(native_authorization.into())])
            .unwrap_err()
            .message()
            .contains("must be mlpl")
    );

    let reordered = format!("{native_authorization}\norder = [\"handler\", \"limits\"]\n");
    assert_eq!(
        middleware_plan_value(&[Value::String(reordered)])
            .unwrap_err()
            .message(),
        "middleware config is malformed"
    );
}
