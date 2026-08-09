use std::mem::{align_of, size_of};
use std::slice;

use mlpl_extension_abi::{AbiArrayView, DTypeTag};

const MAX_RANK: usize = 8;
const MAX_DATA_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DType {
    U8,
    I64,
    F32,
    F64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArrayError {
    UnknownDType(u32),
    InvalidRank(usize),
    NullDescriptor,
    NullShape,
    NullStrides,
    NullData,
    ShapeOverflow,
    StorageLength { expected: usize, actual: usize },
    NonContiguous,
    Misaligned,
    DataTooLong(usize),
    WrongDType { expected: DType, actual: DType },
    RowOutOfBounds(usize),
}

#[derive(Clone, Debug, PartialEq)]
enum Storage {
    U8(Vec<u8>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DenseArray {
    shape: Vec<usize>,
    strides: Vec<isize>,
    storage: Storage,
}

#[derive(Clone, Copy, Debug)]
pub struct ArrayView<'a> {
    array: &'a DenseArray,
}

impl DenseArray {
    /// Constructs a row-major f32 array.
    ///
    /// # Errors
    ///
    /// Rejects invalid ranks, overflowing shapes, and storage-size mismatch.
    pub fn from_f32(shape: Vec<usize>, values: Vec<f32>) -> Result<Self, ArrayError> {
        Self::new(shape, Storage::F32(values))
    }

    /// Constructs a row-major u8 array.
    ///
    /// # Errors
    ///
    /// Rejects invalid ranks, overflowing shapes, and storage-size mismatch.
    pub fn from_u8(shape: Vec<usize>, values: Vec<u8>) -> Result<Self, ArrayError> {
        Self::new(shape, Storage::U8(values))
    }

    /// Constructs a row-major i64 array.
    ///
    /// # Errors
    ///
    /// Rejects invalid ranks, overflowing shapes, and storage-size mismatch.
    pub fn from_i64(shape: Vec<usize>, values: Vec<i64>) -> Result<Self, ArrayError> {
        Self::new(shape, Storage::I64(values))
    }

    /// Constructs a row-major f64 array.
    ///
    /// # Errors
    ///
    /// Rejects invalid ranks, overflowing shapes, and storage-size mismatch.
    pub fn from_f64(shape: Vec<usize>, values: Vec<f64>) -> Result<Self, ArrayError> {
        Self::new(shape, Storage::F64(values))
    }

    fn new(shape: Vec<usize>, storage: Storage) -> Result<Self, ArrayError> {
        validate_rank(shape.len())?;
        let dtype = storage_dtype(&storage);
        let elements = element_count(&shape)?;
        let actual = storage_len(&storage)
            .checked_mul(dtype.size())
            .ok_or(ArrayError::ShapeOverflow)?;
        let expected = elements
            .checked_mul(dtype.size())
            .ok_or(ArrayError::ShapeOverflow)?;
        if actual != expected {
            return Err(ArrayError::StorageLength { expected, actual });
        }
        let strides = contiguous_strides(&shape, dtype.size())?;
        Ok(Self {
            shape,
            strides,
            storage,
        })
    }

    #[must_use]
    pub const fn view(&self) -> ArrayView<'_> {
        ArrayView { array: self }
    }

    pub(crate) fn abi_parts(&self) -> (DTypeTag, *const u8, usize, &[usize], &[isize]) {
        let (tag, data, len) = match &self.storage {
            Storage::U8(values) => (DTypeTag::U8, values.as_ptr(), values.len()),
            Storage::I64(values) => (
                DTypeTag::I64,
                values.as_ptr().cast(),
                values.len() * size_of::<i64>(),
            ),
            Storage::F32(values) => (
                DTypeTag::F32,
                values.as_ptr().cast(),
                values.len() * size_of::<f32>(),
            ),
            Storage::F64(values) => (
                DTypeTag::F64,
                values.as_ptr().cast(),
                values.len() * size_of::<f64>(),
            ),
        };
        (tag, data, len, &self.shape, &self.strides)
    }

    pub(crate) unsafe fn copy_foreign(raw: *const AbiArrayView) -> Result<Self, ArrayError> {
        let descriptor = unsafe { raw.as_ref() }.ok_or(ArrayError::NullDescriptor)?;
        let dtype = decode_dtype(descriptor.dtype)?;
        let rank = descriptor.rank as usize;
        validate_rank(rank)?;
        if descriptor.shape.is_null() {
            return Err(ArrayError::NullShape);
        }
        if descriptor.strides.is_null() {
            return Err(ArrayError::NullStrides);
        }
        let shape = unsafe { slice::from_raw_parts(descriptor.shape, rank) }.to_vec();
        let strides = unsafe { slice::from_raw_parts(descriptor.strides, rank) };
        let expected_strides = contiguous_strides(&shape, dtype.size())?;
        if strides != expected_strides {
            return Err(ArrayError::NonContiguous);
        }
        let expected = element_count(&shape)?
            .checked_mul(dtype.size())
            .ok_or(ArrayError::ShapeOverflow)?;
        if descriptor.data.len > MAX_DATA_BYTES {
            return Err(ArrayError::DataTooLong(descriptor.data.len));
        }
        if descriptor.data.len != expected {
            return Err(ArrayError::StorageLength {
                expected,
                actual: descriptor.data.len,
            });
        }
        if expected > 0 && descriptor.data.data.is_null() {
            return Err(ArrayError::NullData);
        }
        if (descriptor.data.data as usize) % dtype.align() != 0 {
            return Err(ArrayError::Misaligned);
        }
        let storage = unsafe { copy_storage(dtype, descriptor.data.data, expected) };
        Self::new(shape, storage)
    }
}

impl<'a> ArrayView<'a> {
    #[must_use]
    pub fn dtype(self) -> DType {
        storage_dtype(&self.array.storage)
    }
    #[must_use]
    pub fn shape(self) -> &'a [usize] {
        &self.array.shape
    }
    #[must_use]
    pub fn strides(self) -> &'a [isize] {
        &self.array.strides
    }
    /// Returns the flat row-major f32 elements.
    ///
    /// # Errors
    ///
    /// Rejects storage with a dtype other than f32.
    pub fn as_f32(self) -> Result<&'a [f32], ArrayError> {
        match &self.array.storage {
            Storage::F32(values) => Ok(values),
            other => Err(ArrayError::WrongDType {
                expected: DType::F32,
                actual: storage_dtype(other),
            }),
        }
    }

    /// Returns one row of a rank-two f32 array.
    ///
    /// # Errors
    ///
    /// Rejects non-f32 storage, non-matrix shape, overflow, and a row outside
    /// the first dimension.
    pub fn row_f32(self, row: usize) -> Result<&'a [f32], ArrayError> {
        let values = self.as_f32()?;
        if self.array.shape.len() != 2 {
            return Err(ArrayError::InvalidRank(self.array.shape.len()));
        }
        let columns = *self
            .array
            .shape
            .get(1)
            .ok_or(ArrayError::InvalidRank(self.array.shape.len()))?;
        let start = row.checked_mul(columns).ok_or(ArrayError::ShapeOverflow)?;
        let end = start
            .checked_add(columns)
            .ok_or(ArrayError::ShapeOverflow)?;
        values
            .get(start..end)
            .ok_or(ArrayError::RowOutOfBounds(row))
    }
}

impl DType {
    const fn size(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::I64 | Self::F64 => 8,
            Self::F32 => 4,
        }
    }
    const fn align(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::I64 => align_of::<i64>(),
            Self::F32 => align_of::<f32>(),
            Self::F64 => align_of::<f64>(),
        }
    }
}

fn validate_rank(rank: usize) -> Result<(), ArrayError> {
    if rank == 0 || rank > MAX_RANK {
        Err(ArrayError::InvalidRank(rank))
    } else {
        Ok(())
    }
}
fn element_count(shape: &[usize]) -> Result<usize, ArrayError> {
    shape.iter().try_fold(1_usize, |count, dimension| {
        count
            .checked_mul(*dimension)
            .ok_or(ArrayError::ShapeOverflow)
    })
}
fn contiguous_strides(shape: &[usize], item_size: usize) -> Result<Vec<isize>, ArrayError> {
    let mut stride = item_size;
    let mut result = vec![0; shape.len()];
    for (index, dimension) in shape.iter().enumerate().rev() {
        result[index] = isize::try_from(stride).map_err(|_| ArrayError::ShapeOverflow)?;
        stride = stride
            .checked_mul(*dimension)
            .ok_or(ArrayError::ShapeOverflow)?;
    }
    Ok(result)
}
fn storage_dtype(storage: &Storage) -> DType {
    match storage {
        Storage::U8(_) => DType::U8,
        Storage::I64(_) => DType::I64,
        Storage::F32(_) => DType::F32,
        Storage::F64(_) => DType::F64,
    }
}
fn storage_len(storage: &Storage) -> usize {
    match storage {
        Storage::U8(v) => v.len(),
        Storage::I64(v) => v.len(),
        Storage::F32(v) => v.len(),
        Storage::F64(v) => v.len(),
    }
}
fn decode_dtype(raw: u32) -> Result<DType, ArrayError> {
    match raw {
        x if x == DTypeTag::U8 as u32 => Ok(DType::U8),
        x if x == DTypeTag::I64 as u32 => Ok(DType::I64),
        x if x == DTypeTag::F32 as u32 => Ok(DType::F32),
        x if x == DTypeTag::F64 as u32 => Ok(DType::F64),
        other => Err(ArrayError::UnknownDType(other)),
    }
}
unsafe fn copy_storage(dtype: DType, data: *const u8, bytes: usize) -> Storage {
    let source = unsafe { slice::from_raw_parts(data, bytes) };
    match dtype {
        DType::U8 => Storage::U8(source.to_vec()),
        DType::I64 => Storage::I64(decode_chunks(source, i64::from_ne_bytes)),
        DType::F32 => Storage::F32(decode_chunks(source, f32::from_ne_bytes)),
        DType::F64 => Storage::F64(decode_chunks(source, f64::from_ne_bytes)),
    }
}

fn decode_chunks<const N: usize, T>(source: &[u8], decode: fn([u8; N]) -> T) -> Vec<T> {
    source
        .chunks_exact(N)
        .map(|chunk| {
            let mut bytes = [0; N];
            bytes.copy_from_slice(chunk);
            decode(bytes)
        })
        .collect()
}
