# Safe Scalar Conversions

The extension SDK converts ABI V1 scalars into ordinary owned Rust values so
extension business logic does not manipulate tags, unions, pointers, or
lifetimes.

## Author-facing values

`Value` supports nil, bool, signed 64-bit integers, 64-bit floats, UTF-8
strings, and arbitrary bytes. `EncodedValue` owns any string/byte allocation
and exposes a stable ABI view only while the encoder remains alive.
`OwnedError` and `EncodedError` provide the same ownership rule for invalid
argument and ordinary extension failures.

Extension authors construct these safe types without `unsafe`. Future generated
trampolines will decode arguments before calling author functions and encode
their results afterward.

## Host-only foreign copying

The SDK's foreign-copy functions are unsafe because a C pointer cannot prove
that a non-null address is readable. Their safety contract belongs to the host
adapter and generated trampolines, not extension business logic. They copy all
strings, bytes, and messages immediately into owned storage.

Decoding fails closed on:

- unknown value tags or error codes;
- non-zero reserved fields;
- boolean payloads other than zero or one;
- null pointers paired with non-zero lengths;
- payloads larger than 16 MiB;
- invalid UTF-8 and empty error messages.

Empty strings and byte arrays are valid and may use a null pointer with zero
length. Integer minimum/maximum, infinities, and NaN retain their exact scalar
semantics. No array or native-handle claims are made by this slice.

The loader now uses these SDK copies for dynamic and static provider results,
removing its earlier one-off `i64` decoder. General argument marshalling remains
future work.
