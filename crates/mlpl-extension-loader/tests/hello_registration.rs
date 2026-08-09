use std::fs;
use std::path::PathBuf;

use mlpl_extension_loader::{CallError, LoadError, Registry, ResolvedPackage, Value};

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

struct PackageFixture(PathBuf);

impl PackageFixture {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("hello-package-{}-{name}", std::process::id()));
        let native = root.join("native");
        fs::create_dir_all(&native).unwrap();
        fs::copy(hello_library(), native.join("hello-library")).unwrap();
        fs::write(root.join("module.mlpl"), "def u:hello(value) { value }\n").unwrap();
        fs::write(
            root.join("extension.toml"),
            format!(
                "[extension]\nname = \"{name}\"\nversion = \"0.1.0\"\nabi = 1\nmodule = \"module.mlpl\"\nnative_namespace = \"_{name}\"\n\n[[native]]\ntarget = \"test-target\"\nlibrary = \"native/hello-library\"\n"
            ),
        )
        .unwrap();
        Self(root)
    }

    fn resolve(&self) -> ResolvedPackage {
        ResolvedPackage::resolve(&self.0.join("extension.toml"), "test-target").unwrap()
    }
}

impl Drop for PackageFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn independently_built_library_registers_namespaced_functions() {
    let registry = Registry::load(hello_library()).unwrap();
    assert_eq!(registry.extension_name(), "_hello");
    assert_eq!(
        registry.function_names(),
        ["_hello.answer", "_hello.fail", "_hello.panic"]
    );
    assert_eq!(registry.call("_hello.answer", &[]), Ok(Value::I64(42)));
}

#[test]
fn typed_errors_and_panics_do_not_escape_the_boundary() {
    let registry = Registry::load(hello_library()).unwrap();
    assert_eq!(
        registry.call("_hello.fail", &[]),
        Err(CallError::Extension("hello requested failure".into()))
    );
    assert_eq!(
        registry.call("_hello.panic", &[]),
        Err(CallError::ExtensionPanicked)
    );
    assert_eq!(
        registry.call("_hello.answer", &[Value::I64(1)]),
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
        registry.call("_hello.answer", &[]),
        Err(CallError::Inactive("_hello".into()))
    );
    assert!(matches!(
        Registry::load(hello_library().with_file_name("missing-extension")),
        Err(LoadError::Open)
    ));
}

#[test]
fn resolved_package_namespace_must_match_library_descriptor() {
    let hello = PackageFixture::new("hello");
    let registry = Registry::load_package(&hello.resolve()).unwrap();
    assert_eq!(registry.call("_hello.answer", &[]), Ok(Value::I64(42)));

    let other = PackageFixture::new("other");
    assert!(matches!(
        Registry::load_package(&other.resolve()),
        Err(LoadError::NamespaceMismatch { expected, actual })
            if expected == "_other" && actual == "_hello"
    ));
}
