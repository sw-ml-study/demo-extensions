//! Small external extension used to prove the public authoring path.
//!
//! It deliberately exposes only zero-argument integer functions so this slice
//! tests loading and lifecycle without pre-empting the SDK conversion saga.

#[allow(unsafe_code)]
mod export {
    use std::mem::size_of;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::ptr;
    use std::sync::LazyLock;

    use mlpl_extension_abi::{
        ABI_VERSION_V1, AbiErrorV1, AbiSlice, AbiValue, ErrorCode, ExtensionDescriptorV1,
        FunctionDescriptorV1,
    };

    struct SharedFunctions([FunctionDescriptorV1; 3]);
    // SAFETY: every pointer targets immutable static bytes or function code.
    unsafe impl Sync for SharedFunctions {}

    struct SharedDescriptor(ExtensionDescriptorV1);
    // SAFETY: the descriptor and referenced function table are immutable and
    // remain resident for the lifetime of the loaded library.
    unsafe impl Send for SharedDescriptor {}
    // SAFETY: see the `Send` justification; shared access cannot mutate it.
    unsafe impl Sync for SharedDescriptor {}

    static FUNCTIONS: SharedFunctions = SharedFunctions([
        FunctionDescriptorV1::with_invoke(AbiSlice::from_bytes(b"answer"), 0, answer),
        FunctionDescriptorV1::with_invoke(AbiSlice::from_bytes(b"fail"), 0, fail),
        FunctionDescriptorV1::with_invoke(AbiSlice::from_bytes(b"panic"), 0, panic_call),
    ]);

    static METADATA: &[u8] = br#"
[[functions]]
name = "answer"
documentation = "Return the canonical extension answer."
returns = "i64"

[[functions]]
name = "fail"
documentation = "Return a contained extension failure."
returns = "i64"

[[functions]]
name = "panic"
documentation = "Demonstrate containment of an extension panic."
returns = "i64"
"#;

    static DESCRIPTOR: LazyLock<SharedDescriptor> = LazyLock::new(|| {
        SharedDescriptor(ExtensionDescriptorV1 {
            struct_size: u32::try_from(size_of::<ExtensionDescriptorV1>()).unwrap_or(u32::MAX),
            abi_version: ABI_VERSION_V1,
            name: AbiSlice::from_bytes(b"_hello"),
            version: AbiSlice::from_bytes(b"0.1.0"),
            functions: FUNCTIONS.0.as_ptr(),
            function_count: FUNCTIONS.0.len(),
            metadata: AbiSlice::from_bytes(METADATA),
        })
    });

    #[unsafe(no_mangle)]
    pub extern "C" fn sw_mlpl_extension_v1() -> *const ExtensionDescriptorV1 {
        ptr::from_ref(&DESCRIPTOR.0)
    }

    unsafe extern "C" fn answer(
        _arguments: *const AbiValue,
        _argument_count: usize,
        output: *mut AbiValue,
        error: *mut AbiErrorV1,
    ) -> u32 {
        unsafe { run(|| Ok(42), output, error) }
    }

    unsafe extern "C" fn fail(
        _arguments: *const AbiValue,
        _argument_count: usize,
        output: *mut AbiValue,
        error: *mut AbiErrorV1,
    ) -> u32 {
        unsafe { run(|| Err(b"hello requested failure"), output, error) }
    }

    unsafe extern "C" fn panic_call(
        _arguments: *const AbiValue,
        _argument_count: usize,
        output: *mut AbiValue,
        error: *mut AbiErrorV1,
    ) -> u32 {
        unsafe { run(|| panic!("contained hello panic"), output, error) }
    }

    unsafe fn run(
        operation: impl FnOnce() -> Result<i64, &'static [u8]>,
        output: *mut AbiValue,
        error: *mut AbiErrorV1,
    ) -> u32 {
        if output.is_null() || error.is_null() {
            return ErrorCode::InvalidArgument as u32;
        }
        match catch_unwind(AssertUnwindSafe(operation)) {
            Ok(Ok(value)) => unsafe { write_success(value, output, error) },
            Ok(Err(message)) => unsafe { write_error(message, output, error) },
            Err(_) => unsafe { write_panic(output, error) },
        }
    }

    unsafe fn write_success(value: i64, output: *mut AbiValue, error: *mut AbiErrorV1) -> u32 {
        unsafe {
            ptr::write(output, AbiValue::from_i64(value));
            ptr::write(error, AbiErrorV1::none());
        }
        ErrorCode::Ok as u32
    }

    unsafe fn write_error(
        message: &'static [u8],
        output: *mut AbiValue,
        error: *mut AbiErrorV1,
    ) -> u32 {
        unsafe {
            ptr::write(output, AbiValue::nil());
            ptr::write(error, AbiErrorV1::new(ErrorCode::ExtensionFailure, message));
        }
        ErrorCode::ExtensionFailure as u32
    }

    unsafe fn write_panic(output: *mut AbiValue, error: *mut AbiErrorV1) -> u32 {
        unsafe {
            ptr::write(output, AbiValue::nil());
            ptr::write(
                error,
                AbiErrorV1::new(ErrorCode::Panic, b"extension panic contained"),
            );
        }
        ErrorCode::Panic as u32
    }
}

pub use export::sw_mlpl_extension_v1 as static_entry;
