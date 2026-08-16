#![allow(unsafe_code)]

use mlpl_extension_abi::{AbiArrayView, AbiSlice, AbiValue, DTypeTag, ValuePayload, ValueTag};
use mlpl_extension_sdk::{ArrayError, DType, DenseArray, Value, copy_foreign_value};

#[test]
fn owned_n_by_three_array_has_a_call_lifetime_view() {
    let values = [1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0];
    let array = DenseArray::from_f32(vec![2, 3], values.to_vec()).unwrap();
    let view = array.view();
    assert_eq!(view.dtype(), DType::F32);
    assert_eq!(view.shape(), [2, 3]);
    assert_eq!(view.strides(), [12, 4]);
    assert_eq!(view.as_f32().unwrap(), values);
    assert_eq!(view.row_f32(1).unwrap(), [4.0, 5.0, 6.0]);
}

#[test]
fn owned_f64_arrays_have_a_typed_flat_view() {
    let values = [1.0_f64, 2.0, 3.0, 4.0];
    let array = DenseArray::from_f64(vec![2, 2], values.to_vec()).unwrap();
    assert_eq!(array.view().dtype(), DType::F64);
    assert_eq!(array.view().as_f64().unwrap(), values);
    assert!(matches!(
        array.view().as_f32(),
        Err(ArrayError::WrongDType {
            expected: DType::F32,
            actual: DType::F64
        })
    ));
}

#[test]
fn shape_overflow_and_storage_mismatch_fail_closed() {
    assert_eq!(
        DenseArray::from_f32(vec![usize::MAX, 2], Vec::new()),
        Err(ArrayError::ShapeOverflow)
    );
    assert_eq!(
        DenseArray::from_f32(vec![2, 3], vec![0.0; 5]),
        Err(ArrayError::StorageLength {
            expected: 24,
            actual: 20,
        })
    );
}

#[test]
fn malformed_foreign_arrays_reject_rank_stride_alignment_and_bounds() {
    #[repr(align(8))]
    struct Aligned([u8; 25]);
    let data = Aligned([0_u8; 25]);
    let shape = [2_usize, 3];
    let strides = [12_isize, 4];
    let valid = AbiArrayView {
        dtype: DTypeTag::F32 as u32,
        rank: 2,
        data: AbiSlice::from_raw_parts(data.0.as_ptr(), 24),
        shape: shape.as_ptr(),
        strides: strides.as_ptr(),
    };
    assert!(matches!(unsafe { decode(&valid) }, Ok(Value::Array(_))));

    let bad_rank = AbiArrayView { rank: 0, ..valid };
    assert_eq!(
        unsafe { decode(&bad_rank) },
        Err(ArrayError::InvalidRank(0).into())
    );
    let bad_stride_values = [8_isize, 4];
    let bad_stride = AbiArrayView {
        strides: bad_stride_values.as_ptr(),
        ..valid
    };
    assert_eq!(
        unsafe { decode(&bad_stride) },
        Err(ArrayError::NonContiguous.into())
    );
    let misaligned = AbiArrayView {
        data: AbiSlice::from_raw_parts(unsafe { data.0.as_ptr().add(1) }, 24),
        ..valid
    };
    assert_eq!(
        unsafe { decode(&misaligned) },
        Err(ArrayError::Misaligned.into())
    );
}

unsafe fn decode(array: &AbiArrayView) -> Result<Value, mlpl_extension_sdk::ConversionError> {
    let raw = AbiValue {
        tag: ValueTag::DenseArray as u32,
        reserved: 0,
        payload: ValuePayload { array },
    };
    unsafe { copy_foreign_value(&raw) }
}
