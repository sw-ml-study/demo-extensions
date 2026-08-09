use std::collections::BTreeMap;
use std::path::Path;
use std::ptr;

use libloading::Library;
use mlpl_extension_abi::{AbiErrorV1, AbiValue, ExtensionEntryV1, InvokeFnV1, validate_descriptor};
use mlpl_extension_sdk::{EncodedValue, ExtensionMetadata, MetadataError};

use crate::foreign::decode_result;
use crate::{CallError, LoadError, ResolvedPackage, Value};

const ENTRY_SYMBOL: &[u8] = b"sw_mlpl_extension_v1\0";

struct RegisteredFunction {
    arity: usize,
    invoke: InvokeFnV1,
}

struct DynamicGuard {
    _library: Library,
}

enum ProviderGuard {
    #[expect(
        dead_code,
        reason = "variant ownership keeps dynamic callables resident"
    )]
    Dynamic(DynamicGuard),
    Static,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderKind {
    Dynamic,
    Static,
}

pub struct Registry {
    provider: ProviderGuard,
    extension_name: String,
    functions: BTreeMap<String, RegisteredFunction>,
    metadata: ExtensionMetadata,
    active: bool,
}

impl Registry {
    /// Loads and validates one V1 dynamic extension.
    ///
    /// # Errors
    ///
    /// Returns a fail-closed loading error when the library, entry point,
    /// descriptor, or callable table violates the V1 contract.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, LoadError> {
        // SAFETY: all foreign symbols and pointers are validated and copied
        // before registration; the Library remains owned by the Registry.
        unsafe { Self::load_foreign(path.as_ref()) }
    }

    /// Registers a statically linked V1 provider through the same validation
    /// and registry path used for dynamic libraries.
    ///
    /// # Errors
    ///
    /// Returns the same descriptor and registration errors as dynamic loading.
    ///
    /// # Safety
    ///
    /// The entry function must return immutable, readable descriptor storage
    /// and callable code that remain valid for the process lifetime.
    pub unsafe fn load_static(entry: ExtensionEntryV1) -> Result<Self, LoadError> {
        unsafe { Self::from_entry(entry, ProviderGuard::Static) }
    }

    /// Loads the native artifact selected by a validated package manifest.
    ///
    /// # Errors
    ///
    /// Returns normal loading errors and additionally rejects a descriptor
    /// whose private namespace differs from the manifest contract.
    pub fn load_package(package: &ResolvedPackage) -> Result<Self, LoadError> {
        let registry = Self::load(package.library_path())?;
        if registry.extension_name != package.native_namespace() {
            return Err(LoadError::NamespaceMismatch {
                expected: package.native_namespace().to_owned(),
                actual: registry.extension_name,
            });
        }
        Ok(registry)
    }

    unsafe fn load_foreign(path: &Path) -> Result<Self, LoadError> {
        let library = unsafe { Library::new(path) }.map_err(|_| LoadError::Open)?;
        let entry = *unsafe { library.get::<ExtensionEntryV1>(ENTRY_SYMBOL) }
            .map_err(|_| LoadError::MissingEntry)?;
        let provider = ProviderGuard::Dynamic(DynamicGuard { _library: library });
        unsafe { Self::from_entry(entry, provider) }
    }

    unsafe fn from_entry(
        entry: ExtensionEntryV1,
        provider: ProviderGuard,
    ) -> Result<Self, LoadError> {
        let pointer = unsafe { entry() };
        let raw = unsafe { pointer.as_ref() }.ok_or(LoadError::NullDescriptor)?;
        let extension =
            unsafe { validate_descriptor(raw) }.map_err(LoadError::InvalidDescriptor)?;
        let extension_name = extension.name().to_owned();
        let metadata =
            ExtensionMetadata::parse(extension.metadata()).map_err(LoadError::InvalidMetadata)?;
        let exports: Vec<_> = extension
            .functions()
            .iter()
            .map(|function| (function.name(), function.arity() as usize))
            .collect();
        metadata
            .validate_exports(&exports)
            .map_err(LoadError::InvalidMetadata)?;
        let functions = register_functions(&extension_name, extension.functions())?;
        Ok(Self {
            provider,
            extension_name,
            functions,
            metadata,
            active: true,
        })
    }

    #[must_use]
    pub fn extension_name(&self) -> &str {
        &self.extension_name
    }

    #[must_use]
    pub fn provider_kind(&self) -> ProviderKind {
        match &self.provider {
            ProviderGuard::Dynamic(_) => ProviderKind::Dynamic,
            ProviderGuard::Static => ProviderKind::Static,
        }
    }

    #[must_use]
    pub fn function_names(&self) -> Vec<&str> {
        self.functions.keys().map(String::as_str).collect()
    }

    /// Renders deterministic signature and documentation metadata.
    ///
    /// # Errors
    ///
    /// Returns `UnknownFunction` when the qualified function is not declared
    /// by this provider.
    pub fn help(&self, qualified_name: &str) -> Result<String, MetadataError> {
        self.metadata.help(qualified_name)
    }

    /// Renders documentation for an extension-defined native type.
    ///
    /// # Errors
    ///
    /// Returns `UnknownType` when the type is not declared by this provider.
    pub fn type_help(&self, name: &str) -> Result<String, MetadataError> {
        self.metadata.type_help(name)
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    pub const fn deactivate(&mut self) {
        self.active = false;
    }

    /// Invokes a registered function and copies its result into host storage.
    ///
    /// # Errors
    ///
    /// Returns lifecycle, lookup, arity, foreign-result, extension, or panic
    /// errors without exposing pointers owned by the dynamic library.
    pub fn call(&self, name: &str, arguments: &[Value]) -> Result<Value, CallError> {
        if !self.active {
            return Err(CallError::Inactive(self.extension_name.clone()));
        }
        let function = self
            .functions
            .get(name)
            .ok_or_else(|| CallError::UnknownFunction(name.to_owned()))?;
        if arguments.len() != function.arity {
            return Err(CallError::WrongArity {
                expected: function.arity,
                actual: arguments.len(),
            });
        }
        let encoded: Vec<_> = arguments.iter().cloned().map(EncodedValue::new).collect();
        let raw: Vec<_> = encoded.iter().map(|value| *value.as_raw()).collect();
        // SAFETY: the function pointer was copied from a validated descriptor,
        // output pointers reference live host stack values, and the library is
        // retained by self for the complete invocation.
        unsafe { invoke(function.invoke, &raw) }
    }
}

fn register_functions(
    extension: &str,
    functions: &[mlpl_extension_abi::ValidatedFunction],
) -> Result<BTreeMap<String, RegisteredFunction>, LoadError> {
    let mut registered = BTreeMap::new();
    for function in functions {
        let name = format!("{extension}.{}", function.name());
        let invoke = function
            .invoke()
            .ok_or_else(|| LoadError::MissingInvoke(name.clone()))?;
        let callable = RegisteredFunction {
            arity: function.arity() as usize,
            invoke,
        };
        if registered.insert(name.clone(), callable).is_some() {
            return Err(LoadError::DuplicateName(name));
        }
    }
    Ok(registered)
}

unsafe fn invoke(function: InvokeFnV1, arguments: &[AbiValue]) -> Result<Value, CallError> {
    let mut output = AbiValue::nil();
    let mut error = AbiErrorV1::none();
    let pointer = if arguments.is_empty() {
        ptr::null()
    } else {
        arguments.as_ptr()
    };
    let status = unsafe { function(pointer, arguments.len(), &raw mut output, &raw mut error) };
    unsafe { decode_result(status, &output, &error) }
}
