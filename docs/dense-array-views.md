# Dense Array Views

ABI V1 represents a dense numeric array with a dtype tag, rank, byte data
slice, shape pointer, and byte-stride pointer. Supported dtypes are `u8`, `i64`,
`f32`, and `f64`; rank is bounded to 1–8 and data is bounded to 16 MiB.

## Validation contract

The safe SDK rejects unknown dtypes, null descriptors and component pointers,
invalid rank, shape multiplication overflow, byte-length mismatch, incorrect
alignment, and non-contiguous strides. Empty dimensions are supported when the
resulting byte length is zero. V1 intentionally accepts only row-major
contiguous arrays; strided foreign views fail closed rather than being silently
repacked.

Expected byte strides are calculated from the final dimension toward the
first. A contiguous f32 `[2,3]` therefore has shape `[2,3]`, strides `[12,4]`,
and 24 data bytes.

## Ownership and call lifetime

The raw descriptor and its pointers are readable only during boundary
validation. The SDK validates alignment and layout, then copies numeric
elements into typed host-owned vectors. `ArrayView<'a>` borrows a `DenseArray`
and cannot outlive that owned value. Encoder storage retains its typed values,
shape, strides, and boxed ABI descriptor through the complete native call.

This is not an end-to-end zero-copy claim. Current evidence proves one bulk
array crosses one loader call without per-element ABI calls, followed by one
defensive boundary copy. A future zero-copy claim requires the upstream
sw-MLPL borrowing/lifetime hook and measurement against its actual array
storage.

## Acceptance proof

The hello provider exports `_hello.sum_positions(array<f32>[N,3]) -> f64`.
Dynamic and static registries each send six f32 values in one `[2,3]` argument
and receive `21.0`. SDK fixtures separately cover malformed rank, stride,
alignment, size, and overflow cases plus typed row access.

MLPL source still cannot invoke this provider until the upstream static
registry/import slice lands. No `../sw-mlpl` source is changed here.
