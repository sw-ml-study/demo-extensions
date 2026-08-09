use std::collections::HashSet;
use std::mem::size_of;
use std::slice;
use std::str;

use crate::{
    ABI_VERSION_V1, AbiSlice, DescriptorError, ExtensionDescriptorV1, FunctionDescriptorV1,
    ValidatedExtension, ValidatedFunction,
};

const MAX_TEXT_BYTES: usize = 16 * 1024;
const MAX_FUNCTIONS: usize = 1024;

/// Copies and validates a foreign extension descriptor.
///
/// # Errors
///
/// Returns a specific `DescriptorError` when header fields, bounds, text, or
/// function metadata violate the V1 contract.
///
/// # Safety
///
/// Every non-null pointer must reference readable storage for its declared
/// length for this call. The ABI cannot prove that an arbitrary address is
/// readable; violating that foreign-caller precondition is undefined behavior.
pub unsafe fn validate_descriptor(
    raw: &ExtensionDescriptorV1,
) -> Result<ValidatedExtension, DescriptorError> {
    validate_header(raw)?;
    let name = unsafe { copy_text(raw.name, "extension name")? };
    let version = unsafe { copy_text(raw.version, "extension version")? };
    let functions = unsafe { copy_functions(raw)? };
    let metadata = unsafe { copy_optional_text(raw.metadata, "extension metadata")? };
    Ok(ValidatedExtension::new(name, version, functions, metadata))
}

fn validate_header(raw: &ExtensionDescriptorV1) -> Result<(), DescriptorError> {
    let expected =
        u32::try_from(size_of::<ExtensionDescriptorV1>()).expect("descriptor size fits in u32");
    if raw.struct_size != expected {
        return Err(DescriptorError::WrongStructSize(raw.struct_size));
    }
    if raw.abi_version != ABI_VERSION_V1 {
        return Err(DescriptorError::UnsupportedAbi(raw.abi_version));
    }
    validate_function_pointer(raw)
}

fn validate_function_pointer(raw: &ExtensionDescriptorV1) -> Result<(), DescriptorError> {
    if raw.function_count > MAX_FUNCTIONS {
        return Err(DescriptorError::TooManyFunctions(raw.function_count));
    }
    if raw.function_count > 0 && raw.functions.is_null() {
        return Err(DescriptorError::NullFunctions);
    }
    Ok(())
}

unsafe fn copy_text(raw: AbiSlice, field: &'static str) -> Result<String, DescriptorError> {
    let text = unsafe { copy_optional_text(raw, field)? };
    if text.is_empty() {
        return Err(DescriptorError::EmptyText(field));
    }
    Ok(text)
}

unsafe fn copy_optional_text(
    raw: AbiSlice,
    field: &'static str,
) -> Result<String, DescriptorError> {
    if raw.len > MAX_TEXT_BYTES {
        return Err(DescriptorError::TextTooLong(field));
    }
    if raw.len > 0 && raw.data.is_null() {
        return Err(DescriptorError::NullData(field));
    }
    let bytes = if raw.len == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(raw.data, raw.len) }
    };
    let text = str::from_utf8(bytes).map_err(|_| DescriptorError::InvalidUtf8(field))?;
    Ok(text.to_owned())
}

unsafe fn copy_functions(
    raw: &ExtensionDescriptorV1,
) -> Result<Vec<ValidatedFunction>, DescriptorError> {
    if raw.function_count == 0 {
        return Ok(Vec::new());
    }
    let entries = unsafe { slice::from_raw_parts(raw.functions, raw.function_count) };
    let mut names = HashSet::with_capacity(entries.len());
    let mut functions = Vec::with_capacity(entries.len());
    for entry in entries {
        functions.push(unsafe { copy_function(entry, &mut names)? });
    }
    Ok(functions)
}

unsafe fn copy_function(
    raw: &FunctionDescriptorV1,
    names: &mut HashSet<String>,
) -> Result<ValidatedFunction, DescriptorError> {
    if raw.reserved != 0 {
        return Err(DescriptorError::ReservedField("function"));
    }
    let name = unsafe { copy_text(raw.name, "function name")? };
    if !names.insert(name.clone()) {
        return Err(DescriptorError::DuplicateFunction(name));
    }
    Ok(ValidatedFunction::new(name, raw.arity, raw.invoke))
}
