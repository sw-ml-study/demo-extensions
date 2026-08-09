# demo-extensions

`demo-extensions` explores how independently built Rust libraries can add
native capabilities to [sw-MLPL](../sw-mlpl) without adding each domain to the
language runtime. Native 3D visualization is the motivating application; the
current foundation deliberately starts with a headless `hello` extension so
ABI, loading, packaging, ownership, and lifecycle behavior can be tested apart
from GPU and window-system concerns.

The repository currently proves the downstream extension boundary with a real
`.dylib`/`.so` and a Rust host harness. sw-MLPL does not yet expose the registry
and import hooks needed for `use hello` from the REPL, scripts, or compiled
programs. That upstream blocker is documented rather than hidden behind a mock.

## What is here

```text
crates/mlpl-extension-abi/      Versioned C-compatible ABI and validation
crates/mlpl-extension-loader/   Package resolver, dynamic loader, and registry
crates/mlpl-extension-sdk/      Safe author-facing SDK scaffold
extensions/hello/               Rust cdylib, package manifest, and MLPL facade
tests/                          Native mlplunit and structural tests
docs/                           Architecture, contracts, plans, and evidence
```

The hello package demonstrates the intended separation:

- Rust exports private `_hello.answer`, `_hello.fail`, and `_hello.panic`
  functions through one `sw_mlpl_extension_v1` entry point.
- `extension.toml` selects an exact macOS or Linux native artifact and declares
  the public `hello` package separately from the private `_hello` namespace.
- `module.mlpl` is the public MLPL facade. It is tested as ordinary MLPL today
  and will bind to the native namespace once sw-MLPL provides the host hook.

## Prerequisites

- Rust 1.85 or newer.
- [`just`](https://github.com/casey/just) for repository task aliases.
- The adjacent `../sw-mlpl` checkout with `target/release/mlpl-repl` or
  `target/debug/mlpl-repl`, or an absolute `MLPL` override.
- `mlplunit` on `PATH`, an absolute `MLPLUNIT` override, or the adjacent
  `/Users/mike/github/softwarewrighter/mlplunit/bin/mlplunit` checkout used by
  the project scripts.

The scripts only select existing tools; they never install or overwrite them.
Environment overrides must be absolute paths.

## Build and test

Build the workspace and the independently loadable hello library:

```sh
cargo build --workspace
cargo build -p mlpl-extension-hello
```

Run focused Rust or MLPL tests:

```sh
just rust-tests
just tests
just list-tests
```

Run the mandatory pre-commit gate:

```sh
just check
```

The complete gate checks repository layout, `.gitignore`, tracked files,
public/private namespaces, Rust formatting, compilation, clippy, all Rust
tests, native mlplunit tests, and whitespace. The intentional panic test may
print its panic-hook message; the test verifies that the panic is converted to
`ExtensionPanicked` before it can unwind across the C ABI.

To run only the dynamic hello acceptance tests:

```sh
cargo build -p mlpl-extension-hello
cargo test -p mlpl-extension-loader --test hello_registration
cargo test -p mlpl-extension-loader --test manifest_resolution
```

## Current status

The completed foundation proves:

- fixed-layout ABI V1 values, errors, descriptors, and version negotiation;
- bounded fail-closed metadata validation and host-owned metadata copies;
- independent shared-library loading with library lifetime retention;
- namespaced typed success, extension failure, and contained-panic calls;
- deactivation that rejects later calls;
- deterministic manifests, exact target selection, canonical path confinement,
  stable diagnostics, and duplicate/mismatch rejection;
- typed function/default/return and native-type metadata with deterministic
  validation and stable help rendering;
- a public MLPL facade kept separate from private native functions.

General argument marshalling, dense array views, native handles, safe authoring
macros, real unload/hot reload, native 3D, and sw-MLPL language integration are
future work. Dynamic/static provider parity shares one tested registration
path, safe SDK scalar/result copying replaces the loader's original one-off
decoder, and signature metadata is checked against every descriptor. Dense
array views are next.

## Documentation

- [Foundation acceptance](docs/foundation-acceptance.md) — evidence matrix,
  execution-mode status, and limitations.
- [ABI V1](docs/abi-v1.md) — layouts, validation, ownership, and safety rules.
- [Hello extension](docs/hello-extension.md) — dynamic loading and lifecycle
  walkthrough.
- [Safe scalar conversions](docs/sdk-scalars.md) — owned SDK values, errors,
  foreign-copy rules, and malformed-input behavior.
- [Signature metadata](docs/signature-metadata.md) — typed arguments, defaults,
  returns, native types, export validation, and stable help.
- [Extension packages](docs/extension-packages.md) — manifest, platform,
  path-security, and namespace contracts.
- [Development and testing](docs/development.md) — tool resolution, TDD, and
  repository commands.
- [Implementation plan](docs/plan.md) — recommended architecture, capability
  gates, and demo order.
- [Saga queue](docs/sagas.md) — completed foundation and queued implementation
  sagas.
- [Upstream sw-MLPL contract](docs/upstream-contract.md) — the host registry,
  array, handle, event-loop, and deployment capabilities required upstream.
- [Research transcript](docs/sw-mlpl-demo-extensions.txt) — original analysis
  and recommendations that informed the plan.

## Repository workflow

Work is test-driven and tracked with AgentRail sagas. Changes are committed and
pushed directly to `main` after `just check`; this repository does not use
feature branches, pull requests, the `gh` CLI, or GitHub Actions for
publication. See [AGENTS.md](AGENTS.md) for the complete process.

## License

Copyright (c) 2026 Michael A Wright. Distributed under the [MIT License](LICENSE).
