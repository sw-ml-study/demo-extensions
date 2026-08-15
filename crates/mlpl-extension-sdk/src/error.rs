#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConversionError {
    UnknownTag(u32),
    ReservedValue,
    InvalidBool(u8),
    NullData,
    DataTooLong(usize),
    InvalidUtf8,
    ReservedError,
    InvalidErrorCode(u32),
    EmptyError,
    Array(ArrayError),
    NullRecord,
    NullRecordFields,
    TooManyRecordFields(usize),
    EmptyRecordFieldName,
    DuplicateRecordField(String),
    RecordNestingTooDeep,
}

impl From<ArrayError> for ConversionError {
    fn from(error: ArrayError) -> Self {
        Self::Array(error)
    }
}
use crate::ArrayError;
