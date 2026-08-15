//! Versioned C-compatible types shared by sw-MLPL hosts and extensions.
//!
//! Raw foreign data is copied into validated owned metadata before the host
//! registers it. The only pointer dereferences are isolated in `validate`.

mod call;
mod error;
mod model;
#[allow(unsafe_code)]
mod validate;
mod validated;

pub use call::{HostCallError, catch_extension_call};
pub use error::{AbiErrorV1, DescriptorError, ErrorCode};
pub use model::{
    ABI_VERSION_V1, AbiArrayView, AbiField, AbiHandle, AbiRecordView, AbiSlice, AbiValue, DTypeTag,
    ExtensionDescriptorV1, ExtensionEntryV1, FunctionDescriptorV1, InvokeFnV1, ValuePayload,
    ValueTag,
};
pub use validate::validate_descriptor;
pub use validated::{ValidatedExtension, ValidatedFunction};
