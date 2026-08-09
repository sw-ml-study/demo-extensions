//! Safe extension-author facade over the versioned native ABI.
//!
//! Owned encoders keep foreign backing storage alive. Host-only decoders copy
//! raw ABI inputs immediately so extension authors work with ordinary values.

#[allow(unsafe_code)]
mod array;
#[allow(unsafe_code)]
mod decode;
mod encode;
mod error;
mod handle;
mod metadata;
mod value;

pub use array::{ArrayError, ArrayView, DType, DenseArray};
pub use decode::{copy_foreign_error, copy_foreign_value};
pub use encode::{EncodedError, EncodedValue};
pub use error::ConversionError;
pub use handle::{HandleError, HandleRegistry, NativeHandle};
pub use metadata::{ExtensionMetadata, MetadataError};
pub use value::{OwnedError, Value};
