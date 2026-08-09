# Safe SDK Authoring

Extension authors implement ordinary safe handlers with one signature:

```rust
fn answer(arguments: &[Value]) -> Result<Value, OwnedError> {
    Ok(Value::I64(42))
}
```

`export_extension!` receives the extension name/version, one TOML metadata
document, and `(trampoline, exported name, arity, handler)` entries. It
generates the immutable V1 function table, descriptor, C trampolines, panic
containment, argument decoding, result/error encoding, and exported entry
symbol. Handler paths and metadata expressions are written from the generated
module's scope, so crate items use paths such as `crate::answer`.

The macro deliberately covers only stabilized boilerplate. It does not infer
metadata from Rust types, invent array layouts, own native resources, load
libraries, or alter package policy. Descriptor validation remains a host
responsibility, and `ExtensionMetadata` remains the canonical runtime check
that generated descriptor names/arities match documentation.

Encoded output/error backing storage is retained in thread-local SDK storage
after a trampoline returns, long enough for the host to make its required
immediate owned copy. Inputs are copied and validated before safe handlers run.
Panics and author errors become typed ABI errors; they do not unwind across C.

The hello crate is the acceptance example. Its source imports only SDK types,
contains no handwritten `unsafe`, and uses the same generated entry for both
dynamic and statically linked provider tests.
