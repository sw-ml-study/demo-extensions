use std::mem::size_of;
use std::ptr;

pub const ABI_VERSION_V1: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AbiSlice {
    pub data: *const u8,
    pub len: usize,
}

impl AbiSlice {
    #[must_use]
    pub const fn from_bytes(value: &[u8]) -> Self {
        Self {
            data: value.as_ptr(),
            len: value.len(),
        }
    }

    #[must_use]
    pub const fn from_raw_parts(data: *const u8, len: usize) -> Self {
        Self { data, len }
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueTag {
    Nil = 0,
    Bool = 1,
    I64 = 2,
    F64 = 3,
    Utf8 = 4,
    Bytes = 5,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union ValuePayload {
    pub boolean: u8,
    pub integer: i64,
    pub float: f64,
    pub slice: AbiSlice,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AbiValue {
    pub tag: u32,
    pub reserved: u32,
    pub payload: ValuePayload,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FunctionDescriptorV1 {
    pub name: AbiSlice,
    pub arity: u32,
    pub reserved: u32,
}

impl FunctionDescriptorV1 {
    #[must_use]
    pub const fn new(name: AbiSlice, arity: u32) -> Self {
        Self {
            name,
            arity,
            reserved: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ExtensionDescriptorV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub name: AbiSlice,
    pub version: AbiSlice,
    pub functions: *const FunctionDescriptorV1,
    pub function_count: usize,
}

impl ExtensionDescriptorV1 {
    #[must_use]
    pub fn new(name: AbiSlice, version: AbiSlice, functions: &[FunctionDescriptorV1]) -> Self {
        let function_ptr = if functions.is_empty() {
            ptr::null()
        } else {
            functions.as_ptr()
        };
        Self {
            struct_size: u32::try_from(size_of::<Self>()).unwrap_or(u32::MAX),
            abi_version: ABI_VERSION_V1,
            name,
            version,
            functions: function_ptr,
            function_count: functions.len(),
        }
    }
}
