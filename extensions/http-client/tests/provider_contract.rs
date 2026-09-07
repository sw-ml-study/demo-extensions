#![allow(unsafe_code)]

use std::path::PathBuf;

use mlpl_extension_loader::{CallError, ProviderKind, Registry, Value};

fn library() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target/debug/deps");
    path.push(format!(
        "{}mlpl_extension_http_client{}",
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
        unsafe { Registry::load_static(mlpl_extension_http_client::static_entry) }.unwrap(),
    ]
}

#[test]
fn dynamic_and_static_providers_publish_the_same_bounded_api() {
    let registries = registries();
    assert_eq!(registries[0].provider_kind(), ProviderKind::Dynamic);
    assert_eq!(registries[1].provider_kind(), ProviderKind::Static);
    for registry in registries {
        assert_eq!(registry.extension_name(), "_http");
        assert_eq!(
            registry.function_names(),
            ["_http.get", "_http.middleware_plan", "_http.request"]
        );
        assert_eq!(
            registry.help("_http.get").unwrap(),
            "_http.get(url: string) -> record\nPerform one bounded GET with fixed conservative defaults."
        );
        assert_eq!(
            registry.help("_http.request").unwrap(),
            "_http.request(request: record) -> record\nPerform one bounded synchronous HTTP or HTTPS request."
        );
        assert!(matches!(
            registry.call("_http.request", &[Value::String("unsafe".into())]),
            Err(CallError::InvalidArgument(message)) if message == "request must be a record"
        ));
        assert!(matches!(
            registry.call("_http.get", &[Value::String("file:///etc/passwd".into())]),
            Err(CallError::InvalidArgument(message))
                if message == "url must be an absolute HTTP or HTTPS URL"
        ));
    }
}
