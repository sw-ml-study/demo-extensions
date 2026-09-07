use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use mlpl_extension_http_server::{
    close_value, listen_value, local_address_value, next_request_value, respond_value,
};
use mlpl_extension_sdk::Value;

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn policy() -> Value {
    Value::String(include_str!("../middleware.example.toml").into())
}

fn listen() -> Value {
    listen_value(&[
        record([
            ("address", Value::String("127.0.0.1".into())),
            ("port", Value::I64(0)),
            ("max_pending", Value::I64(4)),
        ]),
        policy(),
    ])
    .unwrap()
}

fn address(server: &Value) -> String {
    match local_address_value(std::slice::from_ref(server)).unwrap() {
        Value::String(address) => address,
        other => panic!("expected address, received {other:?}"),
    }
}

fn empty_headers() -> Value {
    Value::Record(BTreeMap::from([("count".into(), Value::I64(0))]))
}

#[test]
fn polls_one_request_and_sends_one_owned_response_without_callback() {
    let server = listen();
    let address = address(&server);
    let client = thread::spawn(move || {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .write_all(
                b"POST /runs?id=7 HTTP/1.1\r\nHost: localhost\r\nOrigin: https://playground.example\r\nAuthorization: Bearer demo-token\r\nContent-Length: 4\r\nConnection: close\r\n\r\nping",
            )
            .unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        response
    });

    let request = next_request_value(&[server.clone(), Value::I64(2_000)]).unwrap();
    let Value::Record(fields) = request else {
        panic!("request must be a record");
    };
    assert_eq!(fields.get("method"), Some(&Value::String("POST".into())));
    assert_eq!(fields.get("path"), Some(&Value::String("/runs".into())));
    assert_eq!(fields.get("query"), Some(&Value::String("id=7".into())));
    assert_eq!(fields.get("body"), Some(&Value::Bytes(b"ping".to_vec())));
    assert_eq!(
        fields.get("bearer_token"),
        Some(&Value::String("demo-token".into()))
    );
    assert_eq!(
        fields.get("authorization_owner"),
        Some(&Value::String("mlpl".into()))
    );
    let request_id = fields.get("id").unwrap().clone();
    assert_eq!(
        respond_value(&[
            server.clone(),
            request_id,
            record([
                ("status", Value::I64(201)),
                ("headers", empty_headers()),
                ("body", Value::Bytes(b"created".to_vec())),
            ]),
        ]),
        Ok(Value::Bool(true))
    );
    assert_eq!(close_value(&[server]), Ok(Value::Bool(true)));

    let response = String::from_utf8(client.join().unwrap()).unwrap();
    assert!(response.starts_with("HTTP/1.1 201 Created\r\n"));
    assert!(response.contains("access-control-allow-origin: https://playground.example\r\n"));
    assert!(response.ends_with("\r\n\r\ncreated"));
}

#[test]
fn timeout_returns_nil_and_closed_handles_fail() {
    let server = listen();
    assert_eq!(
        next_request_value(&[server.clone(), Value::I64(5)]),
        Ok(Value::Nil)
    );
    assert_eq!(
        close_value(std::slice::from_ref(&server)),
        Ok(Value::Bool(true))
    );
    assert!(local_address_value(&[server]).is_err());
}

#[test]
fn cors_preflight_is_completed_before_mlpl_dispatch() {
    let server = listen();
    let address = address(&server);
    let client = thread::spawn(move || {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .write_all(
                b"OPTIONS /runs HTTP/1.1\r\nHost: localhost\r\nOrigin: https://playground.example\r\nAccess-Control-Request-Method: POST\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        String::from_utf8(response).unwrap()
    });

    assert_eq!(
        next_request_value(&[server.clone(), Value::I64(100)]),
        Ok(Value::Nil)
    );
    let response = client.join().unwrap();
    assert!(response.starts_with("HTTP/1.1 204 No Content\r\n"));
    assert!(response.contains("access-control-allow-methods: GET, POST\r\n"));
    assert_eq!(close_value(&[server]), Ok(Value::Bool(true)));
}

#[test]
fn pending_capacity_and_response_ids_fail_closed() {
    let server = listen_value(&[
        record([
            ("address", Value::String("127.0.0.1".into())),
            ("port", Value::I64(0)),
            ("max_pending", Value::I64(1)),
        ]),
        policy(),
    ])
    .unwrap();
    let address = address(&server);
    let client = thread::spawn(move || {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
    });
    let request = next_request_value(&[server.clone(), Value::I64(2_000)]).unwrap();
    assert!(matches!(request, Value::Record(_)));
    assert!(
        next_request_value(&[server.clone(), Value::I64(0)])
            .unwrap_err()
            .message()
            .contains("capacity")
    );
    assert!(
        respond_value(&[
            server.clone(),
            Value::I64(999),
            record([
                ("status", Value::I64(200)),
                ("headers", empty_headers()),
                ("body", Value::Bytes(Vec::new())),
            ]),
        ])
        .is_err()
    );
    assert_eq!(close_value(&[server]), Ok(Value::Bool(true)));
    client.join().unwrap();
}
