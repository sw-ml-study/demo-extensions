use mlpl_extension_abi::ErrorCode;

use crate::{DenseArray, NativeHandle};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(DenseArray),
    Handle(NativeHandle),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedError {
    code: ErrorCode,
    message: String,
}

impl OwnedError {
    #[must_use]
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidArgument,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn extension(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::ExtensionFailure,
            message: message.into(),
        }
    }

    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    pub(crate) const fn new(code: ErrorCode, message: String) -> Self {
        Self { code, message }
    }

    pub(crate) fn into_parts(self) -> (ErrorCode, String) {
        (self.code, self.message)
    }
}
