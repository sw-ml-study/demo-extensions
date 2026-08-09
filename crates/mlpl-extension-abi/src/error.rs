use crate::AbiSlice;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    Ok = 0,
    InvalidArgument = 1,
    ExtensionFailure = 2,
    Panic = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AbiErrorV1 {
    pub code: u32,
    pub reserved: u32,
    pub message: AbiSlice,
}

#[derive(Debug, Eq, PartialEq)]
pub enum DescriptorError {
    WrongStructSize(u32),
    UnsupportedAbi(u32),
    NullData(&'static str),
    TextTooLong(&'static str),
    InvalidUtf8(&'static str),
    EmptyText(&'static str),
    NullFunctions,
    TooManyFunctions(usize),
    ReservedField(&'static str),
    DuplicateFunction(String),
}
