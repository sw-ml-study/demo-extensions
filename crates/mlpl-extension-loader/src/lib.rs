//! Safe validation, loading, registration, and lifecycle management.
//!
//! Dynamic loading retains the library for as long as registered callables can
//! be invoked and copies every foreign result before returning to callers.

mod error;
#[allow(unsafe_code)]
mod foreign;
mod manifest;
#[allow(unsafe_code)]
mod registry;

pub use error::{CallError, LoadError, PackageError};
pub use manifest::{PackageCatalog, ResolvedPackage};
pub use mlpl_extension_sdk::{MetadataError, Value};
pub use registry::{ProviderKind, Registry};
