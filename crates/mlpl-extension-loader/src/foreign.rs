use mlpl_extension_abi::{AbiErrorV1, AbiValue, ErrorCode};
use mlpl_extension_sdk::{copy_foreign_error, copy_foreign_value};

use crate::{CallError, Value};

pub(crate) unsafe fn decode_result(
    status: u32,
    value: &AbiValue,
    error: &AbiErrorV1,
) -> Result<Value, CallError> {
    match status {
        code if code == ErrorCode::Ok as u32 => {
            unsafe { copy_foreign_value(value) }.map_err(|_| CallError::InvalidResult)
        }
        code if code == ErrorCode::Panic as u32 => Err(CallError::ExtensionPanicked),
        code if code == ErrorCode::InvalidArgument as u32
            || code == ErrorCode::ExtensionFailure as u32 =>
        {
            let owned =
                unsafe { copy_foreign_error(error) }.map_err(|_| CallError::InvalidError)?;
            if code == ErrorCode::InvalidArgument as u32 {
                Err(CallError::InvalidArgument(owned.message().to_owned()))
            } else {
                Err(CallError::Extension(owned.message().to_owned()))
            }
        }
        _ => Err(CallError::InvalidError),
    }
}
