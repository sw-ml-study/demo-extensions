# demo-extensions

`demo-extensions` explores how independently built Rust libraries can add
native capabilities to [sw-MLPL](../sw-mlpl) without adding each domain to the
language runtime. Native 3D visualization is the motivating application; the
current foundation deliberately starts with a headless `hello` extension so
ABI, loading, packaging, ownership, and lifecycle behavior can be tested apart
from GPU and window-system concerns.

The wireframe cube is a first-pass proof of concept for an extension supplying
interactive native-3D primitives to MLPL. It prioritizes a truthful public API,
MLPL-owned application behavior, portability, and testability over visual
polish; later iterations can refine rendering, controls, and performance.

The repository proves the downstream extension boundary with a real
`.dylib`/`.so` and a Rust host harness. sw-MLPL now exposes a separate static
scalar registry plus a byte-compatible C-descriptor adapter: both its built-in
`hello:answer()` and this repository's `_hello:answer()` provider are proven
through the interpreter. Arrays, persistent handles, and nested records are now
also proven through the real downstream descriptor. `use hello`, compilation,
dynamic loading, and live native event delivery remain tracked contracts.

## What is here

```text
crates/mlpl-extension-abi/      Versioned C-compatible ABI and validation
crates/mlpl-extension-loader/   Package resolver, dynamic loader, and registry
crates/mlpl-extension-sdk/      Safe author-facing SDK scaffold
crates/mlpl-native3d-scene/      Generic line-scene parser and validation
demos/wireframe-cube/            MLPL-owned bulk-array cube scene
extensions/hello/               Rust cdylib, package manifest, and MLPL facade
extensions/boundary-probe/      Public-SDK array/handle/record host probe
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

Open the opt-in native rotating-cube smoke window:

```sh
just cube-3d
```

The MLPL script generates the scene before the shared wgpu/winit application
opens. Escape or the native close control exits. Cube-specific interactive
controls remain MLPL-owned and require the upstream live extension contracts;
the current window is an honest visual PoC, not the completed live API.

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
- bounded dense numeric arrays with validated dtype, rank, shape, byte strides,
  alignment, storage length, and one-call `[N,3]` acceptance;
- extension-scoped, type-tagged generational handles with stale/cross-extension
  rejection and deterministic resource finalization;
- macro-generated ABI descriptors/trampolines around safe Rust handlers, with
  hello containing no handwritten unsafe code;
- a public MLPL facade kept separate from private native functions.
- a deterministic MLPL wireframe-cube scene with independently adjustable
  dimensions, rotation speed, RGBA line color, and thickness;
- a renderer-neutral Rust line-scene contract that validates bulk `[N,3]`
  positions and `[M,2]` edges before later GPU work.
- a deterministic headless transform, perspective projection, clipping, and
  thick-line raster pipeline with portable PPM evidence.

General argument marshalling, real unload/hot reload, the wgpu/winit window,
and complete sw-MLPL language integration are future work. Dynamic/static provider parity shares one tested registration
path, safe SDK scalar/result copying replaces the loader's original one-off
decoder, signature metadata is checked against every descriptor, and dense
arrays cross the provider boundary as validated host-owned values, and native
resources cross only as opaque numeric capabilities. SDK macro migration and
acceptance are next.

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
- [Dense array views](docs/dense-array-views.md) — layout, validation, ownership,
  call lifetime, and measured copy behavior.
- [Native handles](docs/native-handles.md) — capability identity, generations,
  exhaustion, finalization, and deactivation.
- [SDK acceptance](docs/sdk-acceptance.md) — evidence matrix, upstream static
  registry proof, and remaining integration boundaries.
- [SDK authoring](docs/sdk-authoring.md) — safe handler signature and generated
  descriptor/trampoline contract.
- [C provider host acceptance](docs/c-provider-host-acceptance.md) — direct
  downstream descriptor registration and remaining upstream scope.
- [sw-MLPL data-boundary acceptance](docs/upstream-data-boundary-acceptance.md)
  — real interpreter proof for arrays, handles, records, and invalid values.
- [Extension blockers](docs/extensions-blockers.md) — actionable host
  requirements, dependencies, workarounds, and acceptance gates.
- [Wireframe cube scene](docs/wireframe-cube-scene.md) — array-generated cube,
  generic line-scene schema, controls, validation, and current bridge.
- [Headless wireframe renderer](docs/headless-wireframe-renderer.md) — pure
  transform/projection/clipping pipeline, deterministic evidence, and the
  MLPL-owned interactive boundary.
- [Native window](docs/native-window.md) — opt-in cube command, wgpu/winit
  architecture, macOS/Linux handling, and live-interaction blocker.
- [Wireframe cube acceptance](docs/wireframe-cube-acceptance.md) — evidence
  matrix and deliberately narrow PoC claims.
- [sw-MLPL blockers](docs/sw-mlpl-blockers.md) — exact handles, arrays, events,
  viewer-call, and compiler requirements for MLPL-owned interaction.
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
