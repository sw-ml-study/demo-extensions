use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use mlpl_extension_abi::ABI_VERSION_V1;
use serde::Deserialize;

use crate::PackageError;

#[derive(Deserialize)]
struct Manifest {
    extension: ExtensionSection,
    native: Vec<NativeSection>,
}

#[derive(Deserialize)]
struct ExtensionSection {
    name: String,
    version: String,
    abi: u32,
    module: String,
    native_namespace: String,
}

#[derive(Deserialize)]
struct NativeSection {
    target: String,
    library: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ResolvedPackage {
    name: String,
    version: String,
    native_namespace: String,
    module_path: PathBuf,
    library_path: PathBuf,
}

impl ResolvedPackage {
    /// Resolves one manifest for an exact Rust target triple.
    ///
    /// # Errors
    ///
    /// Rejects unreadable or invalid manifests, ABI/platform mismatches,
    /// unsafe paths, duplicate targets, and missing or escaped artifacts.
    pub fn resolve(path: &Path, target: &str) -> Result<Self, PackageError> {
        let manifest_path = fs::canonicalize(path).map_err(|_| PackageError::ManifestRead)?;
        let root = manifest_path.parent().ok_or(PackageError::ManifestRead)?;
        let text = fs::read_to_string(&manifest_path).map_err(|_| PackageError::ManifestRead)?;
        let manifest: Manifest =
            toml::from_str(&text).map_err(|_| PackageError::InvalidManifest)?;
        validate_extension(&manifest.extension)?;
        validate_platforms(&manifest.native)?;
        let native = select_platform(&manifest.native, target)?;
        let module_path = resolve_file(root, &manifest.extension.module, true)?;
        let library_path = resolve_file(root, &native.library, false)?;
        Ok(Self {
            name: manifest.extension.name,
            version: manifest.extension.version,
            native_namespace: manifest.extension.native_namespace,
            module_path,
            library_path,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub fn native_namespace(&self) -> &str {
        &self.native_namespace
    }

    #[must_use]
    pub fn module_path(&self) -> &Path {
        &self.module_path
    }

    #[must_use]
    pub fn library_path(&self) -> &Path {
        &self.library_path
    }
}

pub struct PackageCatalog;

impl PackageCatalog {
    /// Resolves multiple packages while enforcing unique public names.
    ///
    /// # Errors
    ///
    /// Returns the first deterministic package error or a duplicate-name error.
    pub fn resolve(paths: &[PathBuf], target: &str) -> Result<Vec<ResolvedPackage>, PackageError> {
        let mut names = BTreeSet::new();
        let mut packages = Vec::with_capacity(paths.len());
        for path in paths {
            let package = ResolvedPackage::resolve(path, target)?;
            if !names.insert(package.name.clone()) {
                return Err(PackageError::DuplicateName(package.name));
            }
            packages.push(package);
        }
        Ok(packages)
    }
}

fn validate_extension(extension: &ExtensionSection) -> Result<(), PackageError> {
    if !valid_name(&extension.name) {
        return Err(PackageError::InvalidName(extension.name.clone()));
    }
    if extension.version.is_empty() {
        return Err(PackageError::InvalidManifest);
    }
    if extension.abi != ABI_VERSION_V1 {
        return Err(PackageError::UnsupportedAbi(extension.abi));
    }
    let expected = format!("_{}", extension.name);
    if extension.native_namespace != expected {
        return Err(PackageError::NamespaceMismatch {
            expected,
            actual: extension.native_namespace.clone(),
        });
    }
    Ok(())
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn validate_platforms(native: &[NativeSection]) -> Result<(), PackageError> {
    let mut targets = BTreeSet::new();
    for entry in native {
        if !targets.insert(entry.target.as_str()) {
            return Err(PackageError::DuplicatePlatform(entry.target.clone()));
        }
    }
    Ok(())
}

fn select_platform<'a>(
    native: &'a [NativeSection],
    target: &str,
) -> Result<&'a NativeSection, PackageError> {
    native
        .iter()
        .find(|entry| entry.target == target)
        .ok_or_else(|| {
            let mut available: Vec<_> = native.iter().map(|entry| entry.target.clone()).collect();
            available.sort();
            PackageError::UnsupportedPlatform {
                requested: target.to_owned(),
                available,
            }
        })
}

fn resolve_file(root: &Path, relative: &str, module: bool) -> Result<PathBuf, PackageError> {
    let path = Path::new(relative);
    if relative.is_empty()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(PackageError::UnsafePath(relative.to_owned()));
    }
    let joined = root.join(path);
    let resolved = fs::canonicalize(&joined).map_err(|_| missing(relative, module))?;
    if !resolved.starts_with(root) {
        return Err(PackageError::EscapedPackage(relative.to_owned()));
    }
    Ok(resolved)
}

fn missing(path: &str, module: bool) -> PackageError {
    if module {
        PackageError::MissingModule(path.to_owned())
    } else {
        PackageError::MissingArtifact(path.to_owned())
    }
}
