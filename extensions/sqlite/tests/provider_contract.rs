#![allow(unsafe_code)]

use std::path::PathBuf;

use mlpl_extension_loader::{CallError, ProviderKind, Registry, Value};

fn library() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target/debug/deps");
    path.push(format!(
        "{}mlpl_extension_sqlite{}",
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
        unsafe { Registry::load_static(mlpl_extension_sqlite::static_entry) }.unwrap(),
    ]
}

#[test]
fn dynamic_and_static_providers_publish_the_same_database_api() {
    let registries = registries();
    assert_eq!(registries[0].provider_kind(), ProviderKind::Dynamic);
    assert_eq!(registries[1].provider_kind(), ProviderKind::Static);
    for registry in registries {
        assert_eq!(registry.extension_name(), "_sqlite");
        assert_eq!(
            registry.function_names(),
            [
                "_sqlite.begin",
                "_sqlite.close",
                "_sqlite.commit",
                "_sqlite.execute",
                "_sqlite.open",
                "_sqlite.query",
                "_sqlite.rollback"
            ]
        );
        assert!(matches!(
            registry.call("_sqlite.open", &[Value::String("unsafe".into())]),
            Err(CallError::InvalidArgument(message)) if message == "open config must be a record"
        ));
    }
}
