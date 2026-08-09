use std::path::PathBuf;

use mlpl_extension_loader::{CallError, LoadError, Registry, Value};

fn hello_library() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target/debug/deps");
    path.push(format!(
        "{}mlpl_extension_hello{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    path
}

#[test]
fn independently_built_library_registers_namespaced_functions() {
    let registry = Registry::load(hello_library()).unwrap();
    assert_eq!(registry.extension_name(), "hello");
    assert_eq!(
        registry.function_names(),
        ["hello.answer", "hello.fail", "hello.panic"]
    );
    assert_eq!(registry.call("hello.answer", &[]), Ok(Value::I64(42)));
}

#[test]
fn typed_errors_and_panics_do_not_escape_the_boundary() {
    let registry = Registry::load(hello_library()).unwrap();
    assert_eq!(
        registry.call("hello.fail", &[]),
        Err(CallError::Extension("hello requested failure".into()))
    );
    assert_eq!(
        registry.call("hello.panic", &[]),
        Err(CallError::ExtensionPanicked)
    );
    assert_eq!(
        registry.call("hello.answer", &[Value::I64(1)]),
        Err(CallError::WrongArity {
            expected: 0,
            actual: 1,
        })
    );
}

#[test]
fn registry_retains_library_until_deactivation() {
    let mut registry = Registry::load(hello_library()).unwrap();
    assert!(registry.is_active());
    registry.deactivate();
    assert!(!registry.is_active());
    assert_eq!(
        registry.call("hello.answer", &[]),
        Err(CallError::Inactive("hello".into()))
    );
    assert!(matches!(
        Registry::load(hello_library().with_file_name("missing-extension")),
        Err(LoadError::Open)
    ));
}
