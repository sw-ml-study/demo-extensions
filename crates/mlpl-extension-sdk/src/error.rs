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
}
