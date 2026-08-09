use std::collections::BTreeMap;
use std::path::Path;
use std::ptr;

use libloading::Library;
use mlpl_extension_abi::{AbiErrorV1, AbiValue, ExtensionEntryV1, InvokeFnV1, validate_descriptor};

use crate::foreign::decode_result;
use crate::{CallError, LoadError, Value};

const ENTRY_SYMBOL: &[u8] = b"sw_mlpl_extension_v1\0";

struct RegisteredFunction {
    arity: usize,
    invoke: InvokeFnV1,
}

pub struct Registry {
    _library: Library,
    extension_name: String,
    functions: BTreeMap<String, RegisteredFunction>,
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

    unsafe fn load_foreign(path: &Path) -> Result<Self, LoadError> {
        let library = unsafe { Library::new(path) }.map_err(|_| LoadError::Open)?;
        let entry = unsafe { library.get::<ExtensionEntryV1>(ENTRY_SYMBOL) }
            .map_err(|_| LoadError::MissingEntry)?;
        let pointer = unsafe { entry() };
        let raw = unsafe { pointer.as_ref() }.ok_or(LoadError::NullDescriptor)?;
        let extension =
            unsafe { validate_descriptor(raw) }.map_err(LoadError::InvalidDescriptor)?;
        let extension_name = extension.name().to_owned();
        let functions = register_functions(&extension_name, extension.functions())?;
        Ok(Self {
            _library: library,
            extension_name,
            functions,
            active: true,
        })
    }

    #[must_use]
    pub fn extension_name(&self) -> &str {
        &self.extension_name
    }

    #[must_use]
    pub fn function_names(&self) -> Vec<&str> {
        self.functions.keys().map(String::as_str).collect()
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
        if !arguments.is_empty() {
            return Err(CallError::UnsupportedArguments);
        }
        // SAFETY: the function pointer was copied from a validated descriptor,
        // output pointers reference live host stack values, and the library is
        // retained by self for the complete invocation.
        unsafe { invoke(function.invoke) }
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

unsafe fn invoke(function: InvokeFnV1) -> Result<Value, CallError> {
    let mut output = AbiValue::nil();
    let mut error = AbiErrorV1::none();
    let status = unsafe { function(ptr::null(), 0, &raw mut output, &raw mut error) };
    unsafe { decode_result(status, &output, &error) }
}
