use std::slice;
use std::str;

use mlpl_extension_abi::{AbiErrorV1, AbiSlice, AbiValue, ErrorCode, ValueTag};

use crate::{ConversionError, OwnedError, Value};

const MAX_DATA_BYTES: usize = 16 * 1024 * 1024;

/// Copies one foreign ABI value into safe owned Rust storage.
///
/// # Errors
///
/// Rejects unknown tags, reserved fields, malformed booleans, invalid UTF-8,
/// null/length mismatches, and excessive byte lengths.
///
/// # Safety
///
/// Non-null slice pointers must reference readable storage for their declared
/// length during this call. Generated extension trampolines own this host-only
/// precondition; ordinary extension functions receive `Value` instead.
pub unsafe fn copy_foreign_value(raw: &AbiValue) -> Result<Value, ConversionError> {
    if raw.reserved != 0 {
        return Err(ConversionError::ReservedValue);
    }
    match raw.tag {
        tag if tag == ValueTag::Nil as u32 => Ok(Value::Nil),
        tag if tag == ValueTag::Bool as u32 => unsafe { copy_bool(raw) },
        tag if tag == ValueTag::I64 as u32 => Ok(Value::I64(unsafe { raw.payload.integer })),
        tag if tag == ValueTag::F64 as u32 => Ok(Value::F64(unsafe { raw.payload.float })),
        tag if tag == ValueTag::Utf8 as u32 => unsafe { copy_string(raw) },
        tag if tag == ValueTag::Bytes as u32 => unsafe { copy_bytes(raw).map(Value::Bytes) },
        tag => Err(ConversionError::UnknownTag(tag)),
    }
}

/// Copies one foreign ABI error into safe owned Rust storage.
///
/// # Errors
///
/// Rejects reserved fields, non-error codes, empty messages, malformed slices,
/// and invalid UTF-8.
///
/// # Safety
///
/// A non-null message pointer must reference readable storage for its declared
/// length during this call.
pub unsafe fn copy_foreign_error(raw: &AbiErrorV1) -> Result<OwnedError, ConversionError> {
    if raw.reserved != 0 {
        return Err(ConversionError::ReservedError);
    }
    let code = match raw.code {
        code if code == ErrorCode::InvalidArgument as u32 => ErrorCode::InvalidArgument,
        code if code == ErrorCode::ExtensionFailure as u32 => ErrorCode::ExtensionFailure,
        code if code == ErrorCode::Panic as u32 => ErrorCode::Panic,
        code => return Err(ConversionError::InvalidErrorCode(code)),
    };
    let message = unsafe { copy_text(raw.message)? };
    if message.is_empty() {
        return Err(ConversionError::EmptyError);
    }
    Ok(OwnedError::new(code, message))
}

unsafe fn copy_bool(raw: &AbiValue) -> Result<Value, ConversionError> {
    match unsafe { raw.payload.boolean } {
        0 => Ok(Value::Bool(false)),
        1 => Ok(Value::Bool(true)),
        value => Err(ConversionError::InvalidBool(value)),
    }
}

unsafe fn copy_string(raw: &AbiValue) -> Result<Value, ConversionError> {
    let bytes = unsafe { copy_bytes(raw)? };
    let text = String::from_utf8(bytes).map_err(|_| ConversionError::InvalidUtf8)?;
    Ok(Value::String(text))
}

unsafe fn copy_text(raw: AbiSlice) -> Result<String, ConversionError> {
    let bytes = unsafe { copy_slice(raw)? };
    let text = str::from_utf8(bytes).map_err(|_| ConversionError::InvalidUtf8)?;
    Ok(text.to_owned())
}

unsafe fn copy_bytes(raw: &AbiValue) -> Result<Vec<u8>, ConversionError> {
    let slice = unsafe { raw.payload.slice };
    Ok(unsafe { copy_slice(slice)?.to_vec() })
}

unsafe fn copy_slice<'a>(raw: AbiSlice) -> Result<&'a [u8], ConversionError> {
    if raw.len > MAX_DATA_BYTES {
        return Err(ConversionError::DataTooLong(raw.len));
    }
    if raw.len > 0 && raw.data.is_null() {
        return Err(ConversionError::NullData);
    }
    if raw.len == 0 {
        return Ok(&[]);
    }
    Ok(unsafe { slice::from_raw_parts(raw.data, raw.len) })
}
