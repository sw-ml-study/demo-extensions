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

The safe SDK macro generates exactly one exported symbol,
`sw_mlpl_extension_v1`. Its immutable
descriptor registers three zero-argument functions in the private namespace:

- `_hello.answer` returns the typed integer `42`.
- `_hello.fail` returns a copied, typed extension error.
- `_hello.panic` intentionally panics inside Rust and converts that panic into
  the ABI panic status before returning across `extern "C"`.
- `_hello.sum_positions` accepts one dense f32 `[N,3]` array and returns its
  f64 sum, proving one bulk call through both provider modes.

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

The first three hello functions accept no arguments. `sum_positions` is a
deliberately headless bulk-array proof rather than a visualization claim.
Native handles have a later saga. The library's panic hook may still print the contained panic during a
test, but the test process continues and receives `ExtensionPanicked`.

The package manifest resolves the public `hello` facade separately from the
private `_hello` descriptor. See `extension-packages.md` for platform and path
rules.

sw-MLPL's static scalar registry proves interpreted `hello:answer()` and REPL
description through its in-repo provider. Its C adapter also accepts this
repository's actual `static_entry()` pointer: `upstream_c_provider.rs` proves
`_hello:answer()` and `_hello:fail()` dispatch through the upstream interpreter.
Facade imports, compilation, dynamic loading, arrays, and handles remain
separate integration work.
