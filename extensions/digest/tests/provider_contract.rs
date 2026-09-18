#![allow(unsafe_code)]

use std::path::PathBuf;

use mlpl_extension_loader::{CallError, ProviderKind, Registry, Value};

fn library() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target/debug/deps");
    path.push(format!(
        "{}mlpl_extension_digest{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    path
}

fn registries() -> [Registry; 2] {
    [
        Registry::load(library()).unwrap(),
        // SAFETY: the generated entry returns immutable process-lifetime
        // descriptor storage and callable code.
        unsafe { Registry::load_static(mlpl_extension_digest::static_entry) }.unwrap(),
    ]
}

#[test]
fn dynamic_and_static_providers_publish_the_same_digest_api() {
    let registries = registries();
    assert_eq!(registries[0].provider_kind(), ProviderKind::Dynamic);
    assert_eq!(registries[1].provider_kind(), ProviderKind::Static);
    for registry in registries {
        assert_eq!(registry.extension_name(), "_digest");
        assert_eq!(
            registry.function_names(),
            ["_digest.sha256_bytes", "_digest.sha256_file"]
        );
        assert_eq!(
            registry.help("_digest.sha256_bytes").unwrap(),
            "_digest.sha256_bytes(bytes: bytes) -> string\nHash one in-memory byte string as lowercase hexadecimal."
        );
        assert_eq!(
            registry.help("_digest.sha256_file").unwrap(),
            "_digest.sha256_file(request: record) -> record\nStream an existing confined file and return its digest and size."
        );
        assert!(matches!(
            registry.call("_digest.sha256_bytes", &[Value::String("abc".into())]),
            Err(CallError::InvalidArgument(message))
                if message == "sha256_bytes argument must be bytes"
        ));
        assert!(matches!(
            registry.call("_digest.sha256_file", &[Value::String("abc".into())]),
            Err(CallError::InvalidArgument(message))
                if message == "sha256_file argument must be a record"
        ));
        assert!(matches!(
            registry.call("_digest.sha256_bytes", &[Value::Bytes(b"abc".to_vec())]),
            Ok(Value::String(digest))
                if digest == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        ));
    }
}
