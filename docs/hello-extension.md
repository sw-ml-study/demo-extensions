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
descriptor registers three zero-argument functions in the private namespace:

- `_hello.answer` returns the typed integer `42`.
- `_hello.fail` returns a copied, typed extension error.
- `_hello.panic` intentionally panics inside Rust and converts that panic into
  the ABI panic status before returning across `extern "C"`.

The loader validates and copies descriptor metadata, prefixes callable names
with the extension namespace, retains the `libloading::Library` beside all
function pointers, checks lifecycle and arity before invocation, and copies
results/errors before returning. Deactivation rejects every later call while
the registry continues to own the library until it is dropped.

The descriptor also embeds a TOML metadata document for all three exports.
Registration parses it only after the ABI layer has copied it into host-owned
memory, then requires exact name and arity agreement. For example,
`Registry::help("_hello.answer")` deterministically returns:

```text
_hello.answer() -> i64
Return the canonical extension answer.
```

The same descriptor is also exposed as a statically linked provider. Dynamic
and static providers enter one validation/registration function and run the
identical success, failure, contained-panic, namespace, and deactivation tests.
The provider guard either retains the `Library` or records process-lifetime
static linkage; callable behavior does not branch on provider kind.

## Deliberate limits

The hello functions accept no arguments and return only signed 64-bit values;
the SDK and loader support all scalar copies independently. Dense arrays and
native handles have later sagas. The library's panic hook may still print the contained panic during a
test, but the test process continues and receives `ExtensionPanicked`.

The package manifest resolves the public `hello` facade separately from the
private `_hello` descriptor. See `extension-packages.md` for platform and path
rules.

sw-MLPL does not currently expose a public registry/import API for this
descriptor. Therefore this test proves the downstream ABI, dynamic loader, and
lifecycle model only. It does not claim that `.mlpl` code can invoke the native
function yet.
