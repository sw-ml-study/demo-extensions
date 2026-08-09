use std::mem::size_of;
use std::ptr;

use crate::error::AbiErrorV1;

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
    DenseArray = 6,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DTypeTag {
    U8 = 1,
    I64 = 2,
    F32 = 3,
    F64 = 4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AbiArrayView {
    pub dtype: u32,
    pub rank: u32,
    pub data: AbiSlice,
    pub shape: *const usize,
    pub strides: *const isize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union ValuePayload {
    pub boolean: u8,
    pub integer: i64,
    pub float: f64,
    pub slice: AbiSlice,
    pub array: *const AbiArrayView,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AbiValue {
    pub tag: u32,
    pub reserved: u32,
    pub payload: ValuePayload,
}

impl AbiValue {
    #[must_use]
    pub const fn nil() -> Self {
        Self {
            tag: ValueTag::Nil as u32,
            reserved: 0,
            payload: ValuePayload { integer: 0 },
        }
    }

    #[must_use]
    pub const fn from_i64(value: i64) -> Self {
        Self {
            tag: ValueTag::I64 as u32,
            reserved: 0,
            payload: ValuePayload { integer: value },
        }
    }
}

pub type InvokeFnV1 = unsafe extern "C" fn(
    arguments: *const AbiValue,
    argument_count: usize,
    output: *mut AbiValue,
    error: *mut AbiErrorV1,
) -> u32;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FunctionDescriptorV1 {
    pub name: AbiSlice,
    pub arity: u32,
    pub reserved: u32,
    pub invoke: Option<InvokeFnV1>,
}

impl FunctionDescriptorV1 {
    #[must_use]
    pub const fn new(name: AbiSlice, arity: u32) -> Self {
        Self {
            name,
            arity,
            reserved: 0,
            invoke: None,
        }
    }

    #[must_use]
    pub const fn with_invoke(name: AbiSlice, arity: u32, invoke: InvokeFnV1) -> Self {
        Self {
            name,
            arity,
            reserved: 0,
            invoke: Some(invoke),
        }
    }
}

pub type ExtensionEntryV1 = unsafe extern "C" fn() -> *const ExtensionDescriptorV1;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ExtensionDescriptorV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub name: AbiSlice,
    pub version: AbiSlice,
    pub functions: *const FunctionDescriptorV1,
    pub function_count: usize,
    pub metadata: AbiSlice,
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
            metadata: AbiSlice::from_raw_parts(ptr::null(), 0),
        }
    }

    #[must_use]
    pub const fn with_metadata(mut self, metadata: AbiSlice) -> Self {
        self.metadata = metadata;
        self
    }
}
