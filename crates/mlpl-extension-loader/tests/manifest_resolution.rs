use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use mlpl_extension_loader::{PackageCatalog, PackageError, ResolvedPackage};

static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("demo-extensions-{}-{id}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn package(&self, directory: &str, name: &str, target: &str, library: &str) -> PathBuf {
        let root = self.root.join(directory);
        fs::create_dir_all(root.join("native")).unwrap();
        fs::write(root.join("module.mlpl"), "def u:answer(value) { value }\n").unwrap();
        let manifest = format!(
            "[extension]\nname = \"{name}\"\nversion = \"0.1.0\"\nabi = 1\nmodule = \"module.mlpl\"\nnative_namespace = \"_{name}\"\n\n[[native]]\ntarget = \"{target}\"\nlibrary = \"{library}\"\n"
        );
        fs::write(root.join("extension.toml"), manifest).unwrap();
        root.join("extension.toml")
    }

    fn artifact(&self, package: &str, relative: &str) {
        let path = self.root.join(package).join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"fixture library").unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn resolve(path: &Path, target: &str) -> Result<ResolvedPackage, PackageError> {
    ResolvedPackage::resolve(path, target)
}

#[test]
fn resolves_exact_platform_inside_package_root() {
    let fixture = Fixture::new();
    let manifest = fixture.package("hello", "hello", "test-target", "native/libhello.test");
    fixture.artifact("hello", "native/libhello.test");

    let package = resolve(&manifest, "test-target").unwrap();
    let package_root = fs::canonicalize(fixture.root.join("hello")).unwrap();
    assert_eq!(package.name(), "hello");
    assert_eq!(package.native_namespace(), "_hello");
    assert_eq!(package.module_path(), package_root.join("module.mlpl"));
    assert_eq!(
        package.library_path(),
        package_root.join("native/libhello.test")
    );
}

#[test]
fn traversal_abi_and_missing_artifact_fail_closed() {
    let fixture = Fixture::new();
    let traversal = fixture.package("traversal", "hello", "test-target", "../escape.so");
    assert_eq!(
        resolve(&traversal, "test-target"),
        Err(PackageError::UnsafePath("../escape.so".into()))
    );

    let mismatch = fixture.package("mismatch", "hello", "test-target", "native/lib.so");
    let text = fs::read_to_string(&mismatch)
        .unwrap()
        .replace("abi = 1", "abi = 2");
    fs::write(&mismatch, text).unwrap();
    assert_eq!(
        resolve(&mismatch, "test-target"),
        Err(PackageError::UnsupportedAbi(2))
    );

    let missing = fixture.package("missing", "hello", "test-target", "native/missing.so");
    assert_eq!(
        resolve(&missing, "test-target"),
        Err(PackageError::MissingArtifact("native/missing.so".into()))
    );
}

#[test]
fn platform_and_duplicate_name_diagnostics_are_deterministic() {
    let fixture = Fixture::new();
    let first = fixture.package("first", "hello", "z-target", "native/lib.so");
    fixture.artifact("first", "native/lib.so");
    assert_eq!(
        resolve(&first, "a-target"),
        Err(PackageError::UnsupportedPlatform {
            requested: "a-target".into(),
            available: vec!["z-target".into()],
        })
    );

    let second = fixture.package("second", "hello", "z-target", "native/lib.so");
    fixture.artifact("second", "native/lib.so");
    assert_eq!(
        PackageCatalog::resolve(&[first, second], "z-target"),
        Err(PackageError::DuplicateName("hello".into()))
    );

    let error = PackageError::UnsupportedPlatform {
        requested: "a-target".into(),
        available: vec!["m-target".into(), "z-target".into()],
    };
    assert_eq!(
        error.to_string(),
        "unsupported platform 'a-target'; available: m-target, z-target"
    );
}
