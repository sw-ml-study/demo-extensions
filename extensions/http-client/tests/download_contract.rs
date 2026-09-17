use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::Path;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use mlpl_extension_http_client::download_value;
use mlpl_extension_sdk::Value;
use sha2::{Digest, Sha256};

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn payload(size: usize) -> Vec<u8> {
    (0..size)
        .map(|index| u8::try_from((index * 7 + index / 251) % 256).unwrap())
        .collect()
}

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

fn request(url: String, root: &Path, path: &str, expected: &[u8], timeout_ms: i64) -> Value {
    record([
        ("url", Value::String(url)),
        (
            "expected_bytes",
            Value::I64(i64::try_from(expected.len()).unwrap()),
        ),
        ("sha256", Value::String(hex_sha256(expected))),
        ("root", Value::String(root.to_str().unwrap().to_owned())),
        ("path", Value::String(path.to_owned())),
        ("chunk_bytes", Value::I64(4096)),
        ("timeout_ms", Value::I64(timeout_ms)),
    ])
}

fn with_field(value: Value, name: &str, field: Value) -> Value {
    let Value::Record(mut fields) = value else {
        unreachable!();
    };
    fields.insert(name.to_owned(), field);
    Value::Record(fields)
}

fn read_headers(stream: &mut std::net::TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut request = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let size = stream.read(&mut chunk).unwrap();
        request.extend_from_slice(&chunk[..size]);
        if size == 0 || request.windows(4).any(|window| window == b"\r\n\r\n") {
            return request;
        }
    }
}

fn response(status: &str, headers: &[(&str, String)], body: &[u8]) -> Vec<u8> {
    let mut bytes = format!("HTTP/1.1 {status}\r\n").into_bytes();
    for (name, value) in headers {
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    bytes.extend_from_slice(b"Connection: close\r\n\r\n");
    bytes.extend_from_slice(body);
    bytes
}

fn ok_response(body: &[u8]) -> Vec<u8> {
    response(
        "200 OK",
        &[("Content-Length", body.len().to_string())],
        body,
    )
}

/// Serves each prepared response to one connection, optionally stalling after
/// `stall_after` bytes of the response for `stall` before closing.
fn serve(
    responses: Vec<Vec<u8>>,
    stall: Option<(usize, Duration)>,
) -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        for bytes in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = read_headers(&mut stream);
            match stall {
                Some((sent, pause)) => {
                    stream.write_all(&bytes[..sent]).unwrap();
                    stream.flush().unwrap();
                    thread::sleep(pause);
                }
                None => stream.write_all(&bytes).unwrap(),
            }
        }
    });
    (address, server)
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names = fs::read_dir(dir)
        .map(|iter| {
            iter.map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Record(fields) = value else {
        panic!("result must be a record");
    };
    fields.get(name).unwrap()
}

#[test]
fn download_streams_verifies_and_publishes_atomically() {
    let body = payload(300_000);
    let (address, server) = serve(vec![ok_response(&body)], None);
    let root = tempfile::tempdir().unwrap();

    let result = download_value(&[request(
        format!("http://{address}/model.bin"),
        root.path(),
        "vendor/model/model.bin",
        &body,
        5_000,
    )])
    .unwrap();

    let target = root
        .path()
        .canonicalize()
        .unwrap()
        .join("vendor/model/model.bin");
    assert_eq!(fs::read(&target).unwrap(), body);
    assert_eq!(
        field(&result, "path"),
        &Value::String(target.to_str().unwrap().to_owned())
    );
    assert_eq!(field(&result, "bytes"), &Value::I64(300_000));
    assert_eq!(field(&result, "sha256"), &Value::String(hex_sha256(&body)));
    assert_eq!(field(&result, "reused"), &Value::Bool(false));
    assert_eq!(entries(target.parent().unwrap()), vec!["model.bin"]);
    server.join().unwrap();
}

#[test]
fn truncated_body_leaves_no_partial_file() {
    let body = payload(50_000);
    let mut truncated = response(
        "200 OK",
        &[("Content-Length", body.len().to_string())],
        &body,
    );
    truncated.truncate(truncated.len() - 20_000);
    let (address, server) = serve(vec![truncated], None);
    let root = tempfile::tempdir().unwrap();

    let error = download_value(&[request(
        format!("http://{address}/"),
        root.path(),
        "out/file.bin",
        &body,
        5_000,
    )])
    .unwrap_err();

    assert!(error.message().contains("truncated"), "{}", error.message());
    assert!(entries(&root.path().join("out")).is_empty());
    server.join().unwrap();
}

#[test]
fn tampered_bytes_fail_checksum_and_clean_up() {
    let body = payload(20_000);
    let mut tampered = body.clone();
    tampered[12_345] ^= 0x01;
    let (address, server) = serve(vec![ok_response(&tampered)], None);
    let root = tempfile::tempdir().unwrap();

    let error = download_value(&[request(
        format!("http://{address}/"),
        root.path(),
        "file.bin",
        &body,
        5_000,
    )])
    .unwrap_err();

    assert!(error.message().contains("sha256"), "{}", error.message());
    assert!(entries(root.path()).is_empty());
    server.join().unwrap();
}

#[test]
fn over_delivered_body_without_length_fails_closed() {
    let body = payload(10_000);
    let mut longer = body.clone();
    longer.extend_from_slice(&[0xAA; 64]);
    let (address, server) = serve(vec![response("200 OK", &[], &longer)], None);
    let root = tempfile::tempdir().unwrap();

    let error = download_value(&[request(
        format!("http://{address}/"),
        root.path(),
        "file.bin",
        &body,
        5_000,
    )])
    .unwrap_err();

    assert!(
        error.message().contains("exceeds expected_bytes"),
        "{}",
        error.message()
    );
    assert!(entries(root.path()).is_empty());
    server.join().unwrap();
}

#[test]
fn content_length_mismatch_fails_before_writing() {
    let body = payload(1_000);
    let (address, server) = serve(
        vec![response(
            "200 OK",
            &[("Content-Length", "999".to_owned())],
            &body[..999],
        )],
        None,
    );
    let root = tempfile::tempdir().unwrap();

    let error = download_value(&[request(
        format!("http://{address}/"),
        root.path(),
        "file.bin",
        &body,
        5_000,
    )])
    .unwrap_err();

    assert!(
        error.message().contains("content-length"),
        "{}",
        error.message()
    );
    assert!(entries(root.path()).is_empty());
    server.join().unwrap();
}

#[test]
fn non_success_status_fails_closed() {
    let body = payload(10);
    let (address, server) = serve(
        vec![response(
            "404 Not Found",
            &[("Content-Length", "10".to_owned())],
            &body,
        )],
        None,
    );
    let root = tempfile::tempdir().unwrap();

    let error = download_value(&[request(
        format!("http://{address}/"),
        root.path(),
        "file.bin",
        &body,
        5_000,
    )])
    .unwrap_err();

    assert!(error.message().contains("404"), "{}", error.message());
    assert!(entries(root.path()).is_empty());
    server.join().unwrap();
}

#[test]
fn redirect_limit_fails_closed() {
    let body = payload(10);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    // Detached: the client stops at its redirect budget, so the accept loop is
    // ended by process exit rather than by a fixed connection count.
    thread::spawn(move || {
        while let Ok((mut stream, _)) = listener.accept() {
            let _ = read_headers(&mut stream);
            let _ = stream.write_all(&response(
                "302 Found",
                &[
                    ("Location", format!("http://{address}/loop")),
                    ("Content-Length", "0".to_owned()),
                ],
                b"",
            ));
        }
    });
    let root = tempfile::tempdir().unwrap();

    let error = download_value(&[request(
        format!("http://{address}/loop"),
        root.path(),
        "file.bin",
        &body,
        5_000,
    )])
    .unwrap_err();

    assert!(
        error.message().contains("Too Many Redirects"),
        "{}",
        error.message()
    );
    assert!(entries(root.path()).is_empty());
}

#[test]
fn unfollowable_redirect_status_fails_closed() {
    // A 3xx without Location is returned as a successful call by the
    // transport, so the download path must reject the status itself.
    let body = payload(16);
    let (address, server) = serve(
        vec![response(
            "302 Found",
            &[("Content-Length", "16".to_owned())],
            &body,
        )],
        None,
    );
    let root = tempfile::tempdir().unwrap();

    let error = download_value(&[request(
        format!("http://{address}/"),
        root.path(),
        "file.bin",
        &body,
        5_000,
    )])
    .unwrap_err();

    assert!(
        error.message().contains("HTTP status 302"),
        "{}",
        error.message()
    );
    assert!(entries(root.path()).is_empty());
    server.join().unwrap();
}

#[test]
fn stalled_transfer_times_out_and_cleans_up() {
    let body = payload(8_000);
    let (address, server) = serve(
        vec![ok_response(&body)],
        Some((2_000, Duration::from_millis(1_500))),
    );
    let root = tempfile::tempdir().unwrap();

    let error = download_value(&[request(
        format!("http://{address}/"),
        root.path(),
        "file.bin",
        &body,
        300,
    )])
    .unwrap_err();

    assert!(
        error.message().contains("truncated") || error.message().contains("timed out"),
        "{}",
        error.message()
    );
    assert!(entries(root.path()).is_empty());
    server.join().unwrap();
}

#[test]
fn existing_verified_file_is_reused_without_network() {
    let body = payload(5_000);
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("cache")).unwrap();
    fs::write(root.path().join("cache/file.bin"), &body).unwrap();
    let unreachable = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = unreachable.local_addr().unwrap();
    drop(unreachable);

    let result = download_value(&[request(
        format!("http://{address}/"),
        root.path(),
        "cache/file.bin",
        &body,
        1_000,
    )])
    .unwrap();

    assert_eq!(field(&result, "reused"), &Value::Bool(true));
    assert_eq!(field(&result, "bytes"), &Value::I64(5_000));
    assert_eq!(fs::read(root.path().join("cache/file.bin")).unwrap(), body);
}

#[test]
fn existing_mismatched_file_is_replaced_by_verified_transfer() {
    let body = payload(5_000);
    let (address, server) = serve(vec![ok_response(&body)], None);
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("file.bin"), b"stale contents").unwrap();

    let result = download_value(&[request(
        format!("http://{address}/"),
        root.path(),
        "file.bin",
        &body,
        5_000,
    )])
    .unwrap();

    assert_eq!(field(&result, "reused"), &Value::Bool(false));
    assert_eq!(fs::read(root.path().join("file.bin")).unwrap(), body);
    assert_eq!(entries(root.path()), vec!["file.bin"]);
    server.join().unwrap();
}

#[test]
fn plan_rejects_unsafe_or_unbounded_inputs_before_io() {
    let body = payload(16);
    let root = tempfile::tempdir().unwrap();
    let base = || {
        request(
            "http://127.0.0.1:9/".to_owned(),
            root.path(),
            "file.bin",
            &body,
            1_000,
        )
    };
    let cases: Vec<(Value, &str)> = vec![
        (
            with_field(base(), "url", Value::String("file:///etc/passwd".into())),
            "HTTP or HTTPS",
        ),
        (
            with_field(base(), "root", Value::String("relative/root".into())),
            "root must be an absolute",
        ),
        (
            with_field(
                base(),
                "root",
                Value::String(root.path().join("missing").to_str().unwrap().into()),
            ),
            "root must be an existing directory",
        ),
        (
            with_field(base(), "path", Value::String("../escape.bin".into())),
            "confined relative path",
        ),
        (
            with_field(base(), "path", Value::String("/abs.bin".into())),
            "confined relative path",
        ),
        (
            with_field(base(), "path", Value::String(String::new())),
            "confined relative path",
        ),
        (
            with_field(base(), "expected_bytes", Value::I64(0)),
            "expected_bytes",
        ),
        (
            with_field(base(), "expected_bytes", Value::I64(9 << 30)),
            "expected_bytes",
        ),
        (
            with_field(base(), "sha256", Value::String("abc".into())),
            "sha256 must be 64",
        ),
        (
            with_field(base(), "sha256", Value::String("g".repeat(64))),
            "sha256 must be 64",
        ),
        (
            with_field(base(), "chunk_bytes", Value::I64(0)),
            "chunk_bytes",
        ),
        (
            with_field(base(), "timeout_ms", Value::I64(0)),
            "timeout_ms",
        ),
        (
            with_field(base(), "extra", Value::Bool(true)),
            "fields do not match",
        ),
    ];
    for (value, expected) in cases {
        let error = download_value(&[value]).unwrap_err();
        assert!(
            error.message().contains(expected),
            "expected {expected:?} in {:?}",
            error.message()
        );
    }
    assert!(
        download_value(&[Value::Nil])
            .unwrap_err()
            .message()
            .contains("must be a record")
    );
    assert!(entries(root.path()).is_empty());
}

#[test]
fn uppercase_digest_is_accepted() {
    let body = payload(64);
    let (address, server) = serve(vec![ok_response(&body)], None);
    let root = tempfile::tempdir().unwrap();
    let value = with_field(
        request(
            format!("http://{address}/"),
            root.path(),
            "file.bin",
            &body,
            5_000,
        ),
        "sha256",
        Value::String(hex_sha256(&body).to_ascii_uppercase()),
    );

    let result = download_value(&[value]).unwrap();

    assert_eq!(field(&result, "sha256"), &Value::String(hex_sha256(&body)));
    server.join().unwrap();
}
