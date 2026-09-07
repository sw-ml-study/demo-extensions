#![allow(unsafe_code)]

use std::path::PathBuf;

use mlpl_extension_loader::{CallError, ProviderKind, Registry, Value};

fn library() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target/debug/deps");
    path.push(format!(
        "{}mlpl_extension_http_server{}",
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
        unsafe { Registry::load_static(mlpl_extension_http_server::static_entry) }.unwrap(),
    ]
}

#[test]
fn dynamic_and_static_providers_publish_the_same_callback_free_api() {
    let registries = registries();
    assert_eq!(registries[0].provider_kind(), ProviderKind::Dynamic);
    assert_eq!(registries[1].provider_kind(), ProviderKind::Static);
    for registry in registries {
        assert_eq!(registry.extension_name(), "_web");
        assert_eq!(
            registry.function_names(),
            [
                "_web.close",
                "_web.listen",
                "_web.local_address",
                "_web.next_request",
                "_web.respond"
            ]
        );
        assert!(matches!(
            registry.call("_web.listen", &[Value::Nil, Value::Nil]),
            Err(CallError::InvalidArgument(message))
                if message == "listen expects a config record and middleware TOML"
        ));
    }
}
