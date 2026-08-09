use std::slice;
use std::str;

use mlpl_extension_abi::{AbiErrorV1, AbiSlice, AbiValue, ErrorCode, ValueTag};

use crate::{CallError, Value};

const MAX_ERROR_BYTES: usize = 16 * 1024;

pub(crate) unsafe fn decode_result(
    status: u32,
    value: &AbiValue,
    error: &AbiErrorV1,
) -> Result<Value, CallError> {
    match status {
        code if code == ErrorCode::Ok as u32 => unsafe { copy_value(value) },
        code if code == ErrorCode::Panic as u32 => Err(CallError::ExtensionPanicked),
        code if code == ErrorCode::ExtensionFailure as u32 => {
            Err(CallError::Extension(unsafe { copy_error(error)? }))
        }
        _ => Err(CallError::InvalidError),
    }
}

unsafe fn copy_value(raw: &AbiValue) -> Result<Value, CallError> {
    if raw.reserved != 0 || raw.tag != ValueTag::I64 as u32 {
        return Err(CallError::InvalidResult);
    }
    Ok(Value::I64(unsafe { raw.payload.integer }))
}

unsafe fn copy_error(raw: &AbiErrorV1) -> Result<String, CallError> {
    if raw.reserved != 0 || raw.code != ErrorCode::ExtensionFailure as u32 {
        return Err(CallError::InvalidError);
    }
    unsafe { copy_text(raw.message) }
}

unsafe fn copy_text(raw: AbiSlice) -> Result<String, CallError> {
    if raw.len == 0 || raw.len > MAX_ERROR_BYTES || raw.data.is_null() {
        return Err(CallError::InvalidError);
    }
    let bytes = unsafe { slice::from_raw_parts(raw.data, raw.len) };
    let message = str::from_utf8(bytes).map_err(|_| CallError::InvalidError)?;
    Ok(message.to_owned())
}
