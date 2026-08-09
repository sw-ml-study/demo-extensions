# Hello Dynamic Extension

The hello vertical slice proves an independently built Rust shared library can
be loaded and used through the repository's V1 ABI and safe registry. It is a
host-harness acceptance test, not yet an sw-MLPL REPL integration.

## Build and test

```sh
cargo build -p mlpl-extension-hello
cargo test -p mlpl-extension-loader --test hello_registration
```

On macOS the artifact is a `.dylib`; on Linux it is a `.so`. Tests derive the
platform prefix and suffix through Rust's `DLL_PREFIX` and `DLL_SUFFIX` rather
than hard-coding one operating system.

The library exports exactly one symbol, `sw_mlpl_extension_v1`. Its immutable
descriptor registers three zero-argument functions:

- `hello.answer` returns the typed integer `42`.
- `hello.fail` returns a copied, typed extension error.
- `hello.panic` intentionally panics inside Rust and converts that panic into
  the ABI panic status before returning across `extern "C"`.

The loader validates and copies descriptor metadata, prefixes callable names
with the extension namespace, retains the `libloading::Library` beside all
function pointers, checks lifecycle and arity before invocation, and copies
results/errors before returning. Deactivation rejects every later call while
the registry continues to own the library until it is dropped.

## Deliberate limits

The slice accepts no arguments and decodes only signed 64-bit results. General
value conversion is SDK work; dense arrays and native handles have later
sagas. The library's panic hook may still print the contained panic during a
test, but the test process continues and receives `ExtensionPanicked`.

sw-MLPL does not currently expose a public registry/import API for this
descriptor. Therefore this test proves the downstream ABI, dynamic loader, and
lifecycle model only. It does not claim that `.mlpl` code can invoke the native
function yet.
