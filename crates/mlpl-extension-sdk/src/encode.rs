use mlpl_extension_abi::{
    AbiArrayView, AbiErrorV1, AbiField, AbiHandle, AbiRecordView, AbiSlice, AbiValue, ValuePayload,
    ValueTag,
};

use crate::{OwnedError, Value};

pub struct EncodedValue {
    raw: AbiValue,
    _storage: Option<Box<[u8]>>,
    _array: Option<EncodedArray>,
    _record: Option<EncodedRecord>,
}

struct EncodedArray {
    _value: crate::DenseArray,
    _descriptor: Box<AbiArrayView>,
}

struct EncodedRecord {
    _names: Vec<Box<[u8]>>,
    _values: Vec<EncodedValue>,
    _fields: Box<[AbiField]>,
    _descriptor: Box<AbiRecordView>,
}

impl EncodedValue {
    #[must_use]
    pub fn new(value: Value) -> Self {
        match value {
            Value::Nil => Self::scalar(AbiValue::nil()),
            Value::Bool(value) => Self::scalar(AbiValue {
                tag: ValueTag::Bool as u32,
                reserved: 0,
                payload: ValuePayload {
                    boolean: u8::from(value),
                },
            }),
            Value::I64(value) => Self::scalar(AbiValue::from_i64(value)),
            Value::F64(value) => Self::scalar(AbiValue {
                tag: ValueTag::F64 as u32,
                reserved: 0,
                payload: ValuePayload { float: value },
            }),
            Value::String(value) => Self::with_storage(ValueTag::Utf8, value.into_bytes()),
            Value::Bytes(value) => Self::with_storage(ValueTag::Bytes, value),
            Value::Array(value) => Self::with_array(value),
            Value::Handle(handle) => Self::scalar(AbiValue {
                tag: ValueTag::NativeHandle as u32,
                reserved: 0,
                payload: ValuePayload {
                    handle: AbiHandle {
                        extension_id: handle.extension_id(),
                        type_id: handle.type_id(),
                        slot: handle.slot(),
                        generation: handle.generation(),
                    },
                },
            }),
            Value::Record(fields) => Self::with_record(fields),
        }
    }

    fn scalar(raw: AbiValue) -> Self {
        Self {
            raw,
            _storage: None,
            _array: None,
            _record: None,
        }
    }

    fn with_storage(tag: ValueTag, bytes: Vec<u8>) -> Self {
        let storage = bytes.into_boxed_slice();
        let raw = AbiValue {
            tag: tag as u32,
            reserved: 0,
            payload: ValuePayload {
                slice: AbiSlice::from_bytes(&storage),
            },
        };
        Self {
            raw,
            _storage: Some(storage),
            _array: None,
            _record: None,
        }
    }

    fn with_array(value: crate::DenseArray) -> Self {
        let (dtype, data, len, shape, strides) = value.abi_parts();
        let descriptor = Box::new(AbiArrayView {
            dtype: dtype as u32,
            rank: u32::try_from(shape.len()).unwrap_or(u32::MAX),
            data: AbiSlice::from_raw_parts(data, len),
            shape: shape.as_ptr(),
            strides: strides.as_ptr(),
        });
        let raw = AbiValue {
            tag: ValueTag::DenseArray as u32,
            reserved: 0,
            payload: ValuePayload {
                array: descriptor.as_ref(),
            },
        };
        Self {
            raw,
            _storage: None,
            _array: Some(EncodedArray {
                _value: value,
                _descriptor: descriptor,
            }),
            _record: None,
        }
    }

    fn with_record(fields: std::collections::BTreeMap<String, Value>) -> Self {
        let names: Vec<Box<[u8]>> = fields
            .keys()
            .map(|name| name.as_bytes().to_vec().into_boxed_slice())
            .collect();
        let values: Vec<Self> = fields.into_values().map(Self::new).collect();
        let raw_fields: Box<[AbiField]> = names
            .iter()
            .zip(&values)
            .map(|(name, value)| AbiField {
                name: AbiSlice::from_bytes(name),
                value: *value.as_raw(),
            })
            .collect();
        let descriptor = Box::new(AbiRecordView {
            fields: raw_fields.as_ptr(),
            field_count: raw_fields.len(),
        });
        let raw = AbiValue {
            tag: ValueTag::Record as u32,
            reserved: 0,
            payload: ValuePayload {
                record: descriptor.as_ref(),
            },
        };
        Self {
            raw,
            _storage: None,
            _array: None,
            _record: Some(EncodedRecord {
                _names: names,
                _values: values,
                _fields: raw_fields,
                _descriptor: descriptor,
            }),
        }
    }

    #[must_use]
    pub const fn as_raw(&self) -> &AbiValue {
        &self.raw
    }
}

pub struct EncodedError {
    raw: AbiErrorV1,
    _storage: Box<[u8]>,
}

impl EncodedError {
    #[must_use]
    pub fn new(error: OwnedError) -> Self {
        let (code, message) = error.into_parts();
        let storage = message.into_bytes().into_boxed_slice();
        let raw = AbiErrorV1 {
            code: code as u32,
            reserved: 0,
            message: AbiSlice::from_bytes(&storage),
        };
        Self {
            raw,
            _storage: storage,
        }
    }

    #[must_use]
    pub const fn as_raw(&self) -> &AbiErrorV1 {
        &self.raw
    }
}
