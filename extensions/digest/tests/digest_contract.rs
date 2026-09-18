use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use mlpl_extension_digest::{sha256_bytes_value, sha256_file_value};
use mlpl_extension_sdk::Value;

/// Published SHA-256 vectors. The first two are the NIST one-block and
/// two-block examples; the third is the empty input.
const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const ABC_LONG: &str = "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1";
const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn file_request(root: &Path, path: &str, chunk_bytes: i64) -> Value {
    record([
        ("root", Value::String(root.to_str().unwrap().to_owned())),
        ("path", Value::String(path.to_owned())),
        ("chunk_bytes", Value::I64(chunk_bytes)),
    ])
}

fn digest_of(value: Value) -> String {
    match sha256_bytes_value(&[value]).unwrap() {
        Value::String(digest) => digest,
        other => panic!("expected a string digest, got {other:?}"),
    }
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Record(fields) = value else {
        panic!("result must be a record");
    };
    fields.get(name).unwrap()
}

#[test]
fn bytes_digest_matches_published_vectors() {
    assert_eq!(digest_of(Value::Bytes(b"abc".to_vec())), ABC);
    assert_eq!(
        digest_of(Value::Bytes(
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".to_vec()
        )),
        ABC_LONG
    );
    assert_eq!(digest_of(Value::Bytes(Vec::new())), EMPTY);
}

#[test]
fn bytes_digest_rejects_non_bytes_arguments() {
    assert!(
        sha256_bytes_value(&[Value::String("abc".into())])
            .unwrap_err()
            .message()
            .contains("must be bytes")
    );
    assert!(
        sha256_bytes_value(&[])
            .unwrap_err()
            .message()
            .contains("exactly one argument")
    );
}

#[test]
fn file_digest_matches_the_in_memory_digest_at_every_chunk_size() {
    let root = tempfile::tempdir().unwrap();
    // 4096 + 1 bytes, so a power-of-two chunk never aligns with the end.
    let body = (0..4097)
        .map(|index| u8::try_from((index * 31 + 7) % 256).unwrap())
        .collect::<Vec<_>>();
    fs::create_dir_all(root.path().join("vendor")).unwrap();
    fs::write(root.path().join("vendor/blob.bin"), &body).unwrap();
    let expected = digest_of(Value::Bytes(body.clone()));

    // 1 byte forces the single-byte path; 64 and 4096 straddle the SHA-256
    // block size; 1 MiB reads the file in one call.
    for chunk in [1, 64, 4096, 1024 * 1024] {
        let result =
            sha256_file_value(&[file_request(root.path(), "vendor/blob.bin", chunk)]).unwrap();
        assert_eq!(
            field(&result, "sha256"),
            &Value::String(expected.clone()),
            "chunk {chunk}"
        );
        assert_eq!(field(&result, "bytes"), &Value::I64(4097), "chunk {chunk}");
    }
}

#[test]
fn empty_file_digest_is_the_empty_vector() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("empty.bin"), b"").unwrap();

    let result = sha256_file_value(&[file_request(root.path(), "empty.bin", 4096)]).unwrap();

    assert_eq!(field(&result, "sha256"), &Value::String(EMPTY.to_owned()));
    assert_eq!(field(&result, "bytes"), &Value::I64(0));
}

#[test]
fn file_digest_fails_closed_on_unsafe_or_missing_inputs() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("present.bin"), b"abc").unwrap();
    fs::create_dir_all(root.path().join("a-directory")).unwrap();

    let cases: Vec<(Value, &str)> = vec![
        (
            file_request(root.path(), "../escape.bin", 4096),
            "confined relative path",
        ),
        (
            file_request(root.path(), "/etc/passwd", 4096),
            "confined relative path",
        ),
        (
            file_request(root.path(), "", 4096),
            "confined relative path",
        ),
        (
            file_request(root.path(), "missing.bin", 4096),
            "existing file beneath the root",
        ),
        (
            file_request(root.path(), "a-directory", 4096),
            "does not name a regular file",
        ),
        (
            file_request(Path::new("relative/root"), "present.bin", 4096),
            "root must be an absolute",
        ),
        (
            file_request(&root.path().join("missing-root"), "present.bin", 4096),
            "root must be an existing directory",
        ),
        (file_request(root.path(), "present.bin", 0), "chunk_bytes"),
        (
            file_request(root.path(), "present.bin", 64 * 1024 * 1024),
            "chunk_bytes",
        ),
    ];
    for (value, expected) in cases {
        let error = sha256_file_value(&[value]).unwrap_err();
        assert!(
            error.message().contains(expected),
            "expected {expected:?} in {:?}",
            error.message()
        );
    }

    assert!(
        sha256_file_value(&[Value::Nil])
            .unwrap_err()
            .message()
            .contains("must be a record")
    );
    let mut extra = record([
        ("root", Value::String(root.path().to_str().unwrap().into())),
        ("path", Value::String("present.bin".into())),
        ("chunk_bytes", Value::I64(4096)),
        ("extra", Value::Bool(true)),
    ]);
    if let Value::Record(fields) = &mut extra {
        assert_eq!(fields.len(), 4);
    }
    assert!(
        sha256_file_value(&[extra])
            .unwrap_err()
            .message()
            .contains("fields do not match")
    );
}

#[test]
fn symlink_escaping_the_root_is_rejected() {
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.bin"), b"secret").unwrap();
    let root = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        outside.path().join("secret.bin"),
        root.path().join("link.bin"),
    )
    .unwrap();

    let error = sha256_file_value(&[file_request(root.path(), "link.bin", 4096)]).unwrap_err();

    assert!(
        error.message().contains("escapes the configured root"),
        "{}",
        error.message()
    );
}
