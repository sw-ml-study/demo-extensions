#![allow(unsafe_code)]

use std::collections::BTreeMap;
use std::ptr;

use mlpl_extension_abi::{AbiField, AbiRecordView, AbiSlice, AbiValue, ValuePayload, ValueTag};
use mlpl_extension_sdk::{ConversionError, EncodedValue, Value, copy_foreign_value};

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

#[test]
fn nested_records_roundtrip_with_owned_names_and_values() {
    let expected = record([
        ("count", Value::I64(2)),
        (
            "event",
            record([
                ("kind", Value::String("resize".into())),
                ("width", Value::F64(900.0)),
            ]),
        ),
    ]);
    let copied = {
        let encoded = EncodedValue::new(expected.clone());
        unsafe { copy_foreign_value(encoded.as_raw()) }.unwrap()
    };
    assert_eq!(copied, expected);
}

#[test]
fn malformed_record_pointer_count_names_and_duplicates_fail_closed() {
    let null_raw = AbiValue {
        tag: ValueTag::Record as u32,
        reserved: 0,
        payload: ValuePayload {
            record: ptr::null(),
        },
    };
    assert_eq!(
        unsafe { copy_foreign_value(&null_raw) },
        Err(ConversionError::NullRecord)
    );

    let null_record = AbiRecordView {
        fields: ptr::null(),
        field_count: 1,
    };
    let raw = AbiValue {
        tag: ValueTag::Record as u32,
        reserved: 0,
        payload: ValuePayload {
            record: &raw const null_record,
        },
    };
    assert_eq!(
        unsafe { copy_foreign_value(&raw) },
        Err(ConversionError::NullRecordFields)
    );

    let empty_name = AbiField {
        name: AbiSlice::from_raw_parts(ptr::null(), 0),
        value: AbiValue::from_i64(1),
    };
    let empty_view = AbiRecordView {
        fields: &raw const empty_name,
        field_count: 1,
    };
    let empty_raw = AbiValue {
        tag: ValueTag::Record as u32,
        reserved: 0,
        payload: ValuePayload {
            record: &raw const empty_view,
        },
    };
    assert_eq!(
        unsafe { copy_foreign_value(&empty_raw) },
        Err(ConversionError::EmptyRecordFieldName)
    );

    let invalid_name_bytes = [0xff];
    let invalid_name = AbiField {
        name: AbiSlice::from_bytes(&invalid_name_bytes),
        value: AbiValue::from_i64(1),
    };
    let invalid_view = AbiRecordView {
        fields: &raw const invalid_name,
        field_count: 1,
    };
    let invalid_raw = AbiValue {
        tag: ValueTag::Record as u32,
        reserved: 0,
        payload: ValuePayload {
            record: &raw const invalid_view,
        },
    };
    assert_eq!(
        unsafe { copy_foreign_value(&invalid_raw) },
        Err(ConversionError::InvalidUtf8)
    );

    let fields = [
        AbiField {
            name: AbiSlice::from_bytes(b"same"),
            value: AbiValue::from_i64(1),
        },
        AbiField {
            name: AbiSlice::from_bytes(b"same"),
            value: AbiValue::from_i64(2),
        },
    ];
    let duplicate_view = AbiRecordView {
        fields: fields.as_ptr(),
        field_count: fields.len(),
    };
    let duplicate_raw = AbiValue {
        tag: ValueTag::Record as u32,
        reserved: 0,
        payload: ValuePayload {
            record: &raw const duplicate_view,
        },
    };
    assert_eq!(
        unsafe { copy_foreign_value(&duplicate_raw) },
        Err(ConversionError::DuplicateRecordField("same".into()))
    );
}

#[test]
fn malformed_nested_values_fail_closed() {
    let mut malformed = AbiValue::nil();
    malformed.reserved = 1;
    let field = AbiField {
        name: AbiSlice::from_bytes(b"event"),
        value: malformed,
    };
    let view = AbiRecordView {
        fields: &raw const field,
        field_count: 1,
    };
    let raw = AbiValue {
        tag: ValueTag::Record as u32,
        reserved: 0,
        payload: ValuePayload {
            record: &raw const view,
        },
    };
    assert_eq!(
        unsafe { copy_foreign_value(&raw) },
        Err(ConversionError::ReservedValue)
    );
}

#[test]
fn excessive_field_count_and_nesting_fail_before_dereference() {
    let excessive = AbiRecordView {
        fields: ptr::NonNull::<AbiField>::dangling().as_ptr(),
        field_count: 4097,
    };
    let excessive_raw = AbiValue {
        tag: ValueTag::Record as u32,
        reserved: 0,
        payload: ValuePayload {
            record: &raw const excessive,
        },
    };
    assert_eq!(
        unsafe { copy_foreign_value(&excessive_raw) },
        Err(ConversionError::TooManyRecordFields(4097))
    );

    let mut value = Value::Record(BTreeMap::new());
    for depth in 0..33 {
        value = record([("nested", value), ("depth", Value::I64(depth))]);
    }
    let encoded = EncodedValue::new(value);
    assert_eq!(
        unsafe { copy_foreign_value(encoded.as_raw()) },
        Err(ConversionError::RecordNestingTooDeep)
    );
}
