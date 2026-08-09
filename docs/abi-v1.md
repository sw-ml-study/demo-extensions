# Native Extension ABI V1

The first ABI slice defines layout and validation only. Dynamic library loading,
function pointers, invocation, and registration belong to the next AgentRail
step.

## Raw boundary

All exported records use `#[repr(C)]`; discriminants are fixed-width integers.
`ExtensionDescriptorV1` begins with its byte size and ABI version, followed by
UTF-8 name/version slices, a bounded function descriptor array, and an optional
UTF-8 TOML metadata slice. Values use a numeric tag plus an aligned payload
union. Errors use a numeric code and message slice. Reserved fields must be zero
so V1 can reject accidental layout drift.

Foreign slices are pointer/length pairs. Empty lists use a null pointer and zero
length. Non-empty strings and arrays require non-null readable storage. V1 caps
text at 16 KiB and exported functions at 1,024 before dereferencing payload
arrays.

## Safety and ownership

The C ABI cannot establish whether an arbitrary non-null address is readable.
Calling `validate_descriptor` is therefore unsafe and requires the caller to
guarantee readable storage for every declared range during that call. Null/
length inconsistencies, excessive counts, invalid UTF-8, empty required text,
non-zero reserved fields, duplicate function names, ABI mismatch, and structure
size mismatch are rejected.

Validation immediately copies names, versions, function descriptors, and the
metadata document into owned Rust values. The SDK safely parses that copied
document, and the loader requires it to match descriptor names and arities
exactly before registration. Registration retains no extension-owned metadata
pointers. The narrow pointer-reading implementation lives in `validate.rs`;
the rest of the ABI crate remains safe Rust.

## Panic boundary

`catch_extension_call` converts an extension-returned error into
`HostCallError::Extension` and catches Rust panics as `HostCallError::Panicked`.
This is the inner Rust containment mechanism. The loader must apply it before a
call reaches an `extern "C"` boundary because unwinding across that boundary is
not permitted.

## Current limits

- V1 values model nil, bool, signed 64-bit integers, 64-bit floats, UTF-8,
  bytes, and bounded dense numeric arrays. Native handles are a later contract.
- Function descriptors contain name, arity, and an optional V1 invocation
  trampoline. The loader refuses missing trampolines and metadata/export drift.
- Dense arrays carry a fixed dtype tag, rank, data slice, shape pointer, and
  byte-stride pointer. V1 accepts only bounded contiguous row-major input;
  handles remain a later contract.
- Struct layout is C-compatible, but no claim of end-to-end sw-MLPL integration
  is made. The upstream registry/value hooks remain listed in
  `docs/upstream-contract.md`.
