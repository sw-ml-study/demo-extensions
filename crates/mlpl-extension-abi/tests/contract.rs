#![allow(unsafe_code)]

use std::mem::{align_of, size_of};
use std::ptr;

use mlpl_extension_abi::{
    ABI_VERSION_V1, AbiErrorV1, AbiSlice, AbiValue, DescriptorError, ErrorCode,
    ExtensionDescriptorV1, FunctionDescriptorV1, HostCallError, ValuePayload, ValueTag,
    catch_extension_call, validate_descriptor as validate_raw,
};

fn bytes(value: &[u8]) -> AbiSlice {
    AbiSlice::from_bytes(value)
}

fn function(name: &[u8], arity: u32) -> FunctionDescriptorV1 {
    FunctionDescriptorV1::new(bytes(name), arity)
}

fn descriptor<'a>(
    name: &'a [u8],
    version: &'a [u8],
    functions: &'a [FunctionDescriptorV1],
) -> ExtensionDescriptorV1 {
    ExtensionDescriptorV1::new(bytes(name), bytes(version), functions)
}

fn validate_descriptor(
    raw: &ExtensionDescriptorV1,
) -> Result<mlpl_extension_abi::ValidatedExtension, DescriptorError> {
    // SAFETY: every non-null pointer in these tests comes from live local
    // slices retained for the duration of validation.
    unsafe { validate_raw(raw) }
}

#[test]
fn c_layout_has_stable_tags_and_payload_alignment() {
    assert_eq!(ValueTag::Nil as u32, 0);
    assert_eq!(ValueTag::Bool as u32, 1);
    assert_eq!(ValueTag::I64 as u32, 2);
    assert_eq!(ValueTag::F64 as u32, 3);
    assert_eq!(ValueTag::Utf8 as u32, 4);
    assert_eq!(ValueTag::Bytes as u32, 5);
    assert_eq!(size_of::<AbiSlice>(), size_of::<usize>() * 2);
    assert!(size_of::<AbiValue>() >= size_of::<ValuePayload>() + 8);
    assert!(align_of::<AbiValue>() >= align_of::<ValuePayload>());
    assert_eq!(ErrorCode::Ok as u32, 0);
    assert_eq!(ErrorCode::InvalidArgument as u32, 1);
    assert_eq!(ErrorCode::ExtensionFailure as u32, 2);
    assert_eq!(ErrorCode::Panic as u32, 3);
    assert!(size_of::<AbiErrorV1>() >= size_of::<AbiSlice>() + 8);
}

#[test]
fn version_and_struct_size_must_match_v1() {
    let mut raw = descriptor(b"hello", b"0.1.0", &[]);
    raw.abi_version = ABI_VERSION_V1 + 1;
    assert_eq!(
        validate_descriptor(&raw),
        Err(DescriptorError::UnsupportedAbi(ABI_VERSION_V1 + 1))
    );

    raw.abi_version = ABI_VERSION_V1;
    raw.struct_size = 0;
    assert_eq!(
        validate_descriptor(&raw),
        Err(DescriptorError::WrongStructSize(0))
    );
}

#[test]
fn malformed_slices_and_counts_fail_closed() {
    let mut raw = descriptor(b"hello", b"0.1.0", &[]);
    raw.name = AbiSlice::from_raw_parts(ptr::null(), 5);
    assert_eq!(
        validate_descriptor(&raw),
        Err(DescriptorError::NullData("extension name"))
    );

    raw = descriptor(b"hello", b"0.1.0", &[]);
    raw.function_count = 1;
    assert_eq!(
        validate_descriptor(&raw),
        Err(DescriptorError::NullFunctions)
    );

    raw.function_count = usize::MAX;
    assert_eq!(
        validate_descriptor(&raw),
        Err(DescriptorError::TooManyFunctions(usize::MAX))
    );
}

#[test]
fn invalid_text_and_duplicate_functions_are_rejected() {
    let invalid_name = [0xff];
    let raw = descriptor(&invalid_name, b"0.1.0", &[]);
    assert_eq!(
        validate_descriptor(&raw),
        Err(DescriptorError::InvalidUtf8("extension name"))
    );

    let functions = [function(b"greet", 1), function(b"greet", 2)];
    let raw = descriptor(b"hello", b"0.1.0", &functions);
    assert_eq!(
        validate_descriptor(&raw),
        Err(DescriptorError::DuplicateFunction("greet".into()))
    );

    let mut reserved = function(b"greet", 1);
    reserved.reserved = 1;
    let reserved_functions = [reserved];
    let raw = descriptor(b"hello", b"0.1.0", &reserved_functions);
    assert_eq!(
        validate_descriptor(&raw),
        Err(DescriptorError::ReservedField("function"))
    );
}

#[test]
fn validation_copies_extension_owned_metadata() {
    let validated = {
        let name = Vec::from("hello".as_bytes());
        let version = Vec::from("0.1.0".as_bytes());
        let function_name = Vec::from("greet".as_bytes());
        let functions = [function(&function_name, 1)];
        validate_descriptor(&descriptor(&name, &version, &functions)).unwrap()
    };

    assert_eq!(validated.name(), "hello");
    assert_eq!(validated.version(), "0.1.0");
    assert_eq!(validated.functions()[0].name(), "greet");
    assert_eq!(validated.functions()[0].arity(), 1);
}

#[test]
fn extension_panics_are_contained() {
    let outcome = catch_extension_call(|| -> Result<i64, &'static str> {
        panic!("extension panic must not unwind into the host")
    });
    assert_eq!(outcome, Err(HostCallError::Panicked));

    assert_eq!(catch_extension_call(|| Ok::<_, &str>(42)), Ok(42));
    assert_eq!(
        catch_extension_call(|| Err::<i64, _>("extension error")),
        Err(HostCallError::Extension("extension error"))
    );
}
