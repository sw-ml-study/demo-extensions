use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::slice;

pub use mlpl_extension_abi as abi;
use mlpl_extension_abi::{AbiErrorV1, AbiValue, ErrorCode};

use crate::{EncodedError, EncodedValue, OwnedError, Value, copy_foreign_value};

thread_local! {
    static OUTPUT: RefCell<Option<EncodedValue>> = const { RefCell::new(None) };
    static ERROR: RefCell<Option<EncodedError>> = const { RefCell::new(None) };
}

/// Invokes one safe author handler behind the raw V1 trampoline.
///
/// # Safety
///
/// Argument storage must be readable for `argument_count` values and output
/// pointers must be writable for the duration of this call.
pub unsafe fn invoke(
    handler: fn(&[Value]) -> Result<Value, OwnedError>,
    arguments: *const AbiValue,
    argument_count: usize,
    output: *mut AbiValue,
    error: *mut AbiErrorV1,
) -> u32 {
    if output.is_null() || error.is_null() || (argument_count > 0 && arguments.is_null()) {
        return ErrorCode::InvalidArgument as u32;
    }
    let raw_arguments = if argument_count == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(arguments, argument_count) }
    };
    let decoded: Result<Vec<_>, _> = raw_arguments
        .iter()
        .map(|value| unsafe { copy_foreign_value(value) })
        .collect();
    let result = match decoded {
        Ok(values) => catch_unwind(AssertUnwindSafe(|| handler(&values))),
        Err(_) => {
            return write_error(
                OwnedError::invalid_argument("invalid extension argument"),
                output,
                error,
            );
        }
    };
    match result {
        Ok(Ok(value)) => write_value(value, output, error),
        Ok(Err(failure)) => write_error(failure, output, error),
        Err(_) => write_error(
            OwnedError::new(ErrorCode::Panic, "extension panic contained".into()),
            output,
            error,
        ),
    }
}

fn write_value(value: Value, output: *mut AbiValue, error: *mut AbiErrorV1) -> u32 {
    OUTPUT.with_borrow_mut(|storage| {
        let encoded = EncodedValue::new(value);
        let raw = *encoded.as_raw();
        *storage = Some(encoded);
        unsafe { ptr::write(output, raw) };
    });
    unsafe { ptr::write(error, AbiErrorV1::none()) };
    ErrorCode::Ok as u32
}

fn write_error(failure: OwnedError, output: *mut AbiValue, error: *mut AbiErrorV1) -> u32 {
    let code = failure.code();
    ERROR.with_borrow_mut(|storage| {
        let encoded = EncodedError::new(failure);
        let raw = *encoded.as_raw();
        *storage = Some(encoded);
        unsafe { ptr::write(error, raw) };
    });
    unsafe { ptr::write(output, AbiValue::nil()) };
    code as u32
}
