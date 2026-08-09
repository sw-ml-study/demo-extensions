//! Safe extension-author facade over the versioned native ABI.
//!
//! Owned encoders keep foreign backing storage alive. Host-only decoders copy
//! raw ABI inputs immediately so extension authors work with ordinary values.

#[doc(hidden)]
#[allow(unsafe_code)]
pub mod __private;
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

/// Declares a V1 extension descriptor and C trampolines around safe handlers.
#[macro_export]
macro_rules! export_extension {
    (
        module: $module:ident,
        entry: $entry:ident,
        name: $name:literal,
        version: $version:literal,
        metadata: $metadata:expr,
        functions: [$(($trampoline:ident, $function_name:literal, $arity:expr, $handler:path)),+ $(,)?]
    ) => {
        #[allow(unsafe_code)]
        mod $module {
            use std::sync::LazyLock;
            use $crate::__private::abi::{AbiErrorV1, AbiSlice, AbiValue, ExtensionDescriptorV1, FunctionDescriptorV1};

            struct SharedFunctions([FunctionDescriptorV1; $crate::export_extension!(@count $($trampoline),+)]);
            unsafe impl Sync for SharedFunctions {}
            struct SharedDescriptor(ExtensionDescriptorV1);
            unsafe impl Send for SharedDescriptor {}
            unsafe impl Sync for SharedDescriptor {}

            $(
                unsafe extern "C" fn $trampoline(
                    arguments: *const AbiValue,
                    argument_count: usize,
                    output: *mut AbiValue,
                    error: *mut AbiErrorV1,
                ) -> u32 {
                    unsafe { $crate::__private::invoke($handler, arguments, argument_count, output, error) }
                }
            )+

            static FUNCTIONS: SharedFunctions = SharedFunctions([
                $(FunctionDescriptorV1::with_invoke(AbiSlice::from_bytes($function_name.as_bytes()), $arity, $trampoline)),+
            ]);
            static DESCRIPTOR: LazyLock<SharedDescriptor> = LazyLock::new(|| {
                SharedDescriptor(
                    ExtensionDescriptorV1::new(
                        AbiSlice::from_bytes($name.as_bytes()),
                        AbiSlice::from_bytes($version.as_bytes()),
                        &FUNCTIONS.0,
                    )
                    .with_metadata(AbiSlice::from_bytes(($metadata).as_bytes())),
                )
            });

            #[unsafe(no_mangle)]
            pub extern "C" fn $entry() -> *const ExtensionDescriptorV1 {
                std::ptr::from_ref(&DESCRIPTOR.0)
            }
        }

        pub use $module::$entry;
    };
    (@count $head:ident $(, $tail:ident)*) => { 1usize $(+ { let _ = stringify!($tail); 1usize })* };
}
