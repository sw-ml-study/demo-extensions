#![allow(unsafe_code)]

use std::ptr;

use mlpl_extension_abi::{AbiErrorV1, AbiSlice, AbiValue, ErrorCode, ValuePayload, ValueTag};
use mlpl_extension_sdk::{
    ConversionError, EncodedError, EncodedValue, OwnedError, Value, copy_foreign_error,
    copy_foreign_value,
};

fn roundtrip(value: Value) -> Value {
    let encoded = EncodedValue::new(value);
    // SAFETY: EncodedValue retains every referenced allocation during copying.
    unsafe { copy_foreign_value(encoded.as_raw()) }.unwrap()
}

#[test]
fn scalar_values_roundtrip_at_boundaries() {
    assert_eq!(roundtrip(Value::Nil), Value::Nil);
    assert_eq!(roundtrip(Value::Bool(false)), Value::Bool(false));
    assert_eq!(roundtrip(Value::Bool(true)), Value::Bool(true));
    assert_eq!(roundtrip(Value::I64(i64::MIN)), Value::I64(i64::MIN));
    assert_eq!(roundtrip(Value::I64(i64::MAX)), Value::I64(i64::MAX));
    assert_eq!(
        roundtrip(Value::F64(f64::INFINITY)),
        Value::F64(f64::INFINITY)
    );
    assert!(matches!(roundtrip(Value::F64(f64::NAN)), Value::F64(value) if value.is_nan()));
}

#[test]
fn strings_bytes_and_errors_are_copied_into_owned_storage() {
    let copied_text = {
        let encoded = EncodedValue::new(Value::String("héllo".into()));
        unsafe { copy_foreign_value(encoded.as_raw()) }.unwrap()
    };
    assert_eq!(copied_text, Value::String("héllo".into()));

    let copied_bytes = {
        let encoded = EncodedValue::new(Value::Bytes(vec![0, 1, 0xff]));
        unsafe { copy_foreign_value(encoded.as_raw()) }.unwrap()
    };
    assert_eq!(copied_bytes, Value::Bytes(vec![0, 1, 0xff]));

    let copied_error = {
        let encoded = EncodedError::new(OwnedError::extension("owned failure"));
        unsafe { copy_foreign_error(encoded.as_raw()) }.unwrap()
    };
    assert_eq!(copied_error, OwnedError::extension("owned failure"));

    let argument_error = EncodedError::new(OwnedError::invalid_argument("expected scalar"));
    assert_eq!(
        unsafe { copy_foreign_error(argument_error.as_raw()) },
        Ok(OwnedError::invalid_argument("expected scalar"))
    );
}

#[test]
fn malformed_tags_reserved_fields_and_bools_fail_closed() {
    let mut raw = AbiValue::nil();
    raw.tag = 99;
    assert_eq!(
        unsafe { copy_foreign_value(&raw) },
        Err(ConversionError::UnknownTag(99))
    );

    raw = AbiValue::nil();
    raw.reserved = 1;
    assert_eq!(
        unsafe { copy_foreign_value(&raw) },
        Err(ConversionError::ReservedValue)
    );

    raw = AbiValue {
        tag: ValueTag::Bool as u32,
        reserved: 0,
        payload: ValuePayload { boolean: 2 },
    };
    assert_eq!(
        unsafe { copy_foreign_value(&raw) },
        Err(ConversionError::InvalidBool(2))
    );
}

#[test]
fn malformed_foreign_slices_fail_closed() {
    let null_text = AbiValue {
        tag: ValueTag::Utf8 as u32,
        reserved: 0,
        payload: ValuePayload {
            slice: AbiSlice::from_raw_parts(ptr::null(), 1),
        },
    };
    assert_eq!(
        unsafe { copy_foreign_value(&null_text) },
        Err(ConversionError::NullData)
    );

    let invalid = [0xff];
    let invalid_text = AbiValue {
        tag: ValueTag::Utf8 as u32,
        reserved: 0,
        payload: ValuePayload {
            slice: AbiSlice::from_bytes(&invalid),
        },
    };
    assert_eq!(
        unsafe { copy_foreign_value(&invalid_text) },
        Err(ConversionError::InvalidUtf8)
    );

    let empty_bytes = AbiValue {
        tag: ValueTag::Bytes as u32,
        reserved: 0,
        payload: ValuePayload {
            slice: AbiSlice::from_raw_parts(ptr::null(), 0),
        },
    };
    assert_eq!(
        unsafe { copy_foreign_value(&empty_bytes) },
        Ok(Value::Bytes(vec![]))
    );

    let oversized = AbiValue {
        tag: ValueTag::Bytes as u32,
        reserved: 0,
        payload: ValuePayload {
            slice: AbiSlice::from_raw_parts(
                ptr::NonNull::<u8>::dangling().as_ptr(),
                16 * 1024 * 1024 + 1,
            ),
        },
    };
    assert_eq!(
        unsafe { copy_foreign_value(&oversized) },
        Err(ConversionError::DataTooLong(16 * 1024 * 1024 + 1))
    );
}

#[test]
fn malformed_foreign_errors_fail_closed() {
    let mut raw = AbiErrorV1::new(ErrorCode::ExtensionFailure, b"failure");
    raw.reserved = 1;
    assert_eq!(
        unsafe { copy_foreign_error(&raw) },
        Err(ConversionError::ReservedError)
    );

    raw = AbiErrorV1::new(ErrorCode::Ok, b"not an error");
    assert_eq!(
        unsafe { copy_foreign_error(&raw) },
        Err(ConversionError::InvalidErrorCode(ErrorCode::Ok as u32))
    );

    raw = AbiErrorV1 {
        code: ErrorCode::ExtensionFailure as u32,
        reserved: 0,
        message: AbiSlice::from_raw_parts(ptr::null(), 1),
    };
    assert_eq!(
        unsafe { copy_foreign_error(&raw) },
        Err(ConversionError::NullData)
    );

    raw = AbiErrorV1::new(ErrorCode::ExtensionFailure, b"");
    assert_eq!(
        unsafe { copy_foreign_error(&raw) },
        Err(ConversionError::EmptyError)
    );

    let invalid = [0xff];
    raw = AbiErrorV1 {
        code: ErrorCode::ExtensionFailure as u32,
        reserved: 0,
        message: AbiSlice::from_bytes(&invalid),
    };
    assert_eq!(
        unsafe { copy_foreign_error(&raw) },
        Err(ConversionError::InvalidUtf8)
    );
}
