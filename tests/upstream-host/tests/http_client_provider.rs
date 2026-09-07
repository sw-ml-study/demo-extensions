//! Exercises the HTTP client from MLPL through sw-MLPL's public provider hook.

#![allow(unsafe_code)]

use std::io::{Read, Write};
use std::net::TcpListener;

use mlpl_eval::{Environment, Value};
use mlpl_extension_cabi::{ExtensionDescriptorV1, register_c_extension};

#[test]
fn mlpl_get_reaches_a_deterministic_loopback_server() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 1024];
        let count = stream.read(&mut request).unwrap();
        assert!(String::from_utf8_lossy(&request[..count]).starts_with("GET /clock HTTP/1.1"));
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\n12:34:56Z",
            )
            .unwrap();
    });

    let descriptor = mlpl_extension_http_client::static_entry();
    // SAFETY: the downstream static descriptor and its code remain resident
    // for the duration of this process.
    unsafe { register_c_extension(descriptor.cast::<ExtensionDescriptorV1>()) }.unwrap();

    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../demos/http-client/get.mlpl"),
    )
    .unwrap();
    let tokens = mlpl_parser::lex(&source).unwrap();
    let statements = mlpl_parser::parse(&tokens).unwrap();
    let mut environment = Environment::new();
    environment.cli_args = vec![format!("http://{address}/clock")];
    let value = mlpl_eval::eval_program_value(&statements, &mut environment).unwrap();
    server.join().unwrap();

    match value {
        Value::Array(array) => assert_eq!(array.data(), &[200.0, 9.0]),
        other => panic!("expected MLPL result array, received {other:?}"),
    }
}
