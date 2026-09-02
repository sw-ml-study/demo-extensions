# demo-extensions

`demo-extensions` explores how independently built Rust libraries can add
native capabilities to [sw-MLPL](../sw-mlpl) without adding each domain to the
language runtime. The repository now includes the original headless `hello`
extension foundation plus native interactive line-rendered applications,
bounded model/media/filesystem demonstrations, and a Rust/Yew ML microscope.

The wireframe cube remains the smallest visual teaching path. All interactive
native demos now initialize one retained scene, use stable-ID patches for
geometry/style changes, and use view updates for camera/help/status changes.

The repository proves the downstream extension boundary with a real
`.dylib`/`.so` and a Rust host harness. sw-MLPL now exposes a separate static
scalar registry plus a byte-compatible C-descriptor adapter: both its built-in
`hello:answer()` and this repository's `_hello:answer()` provider are proven
through the interpreter. Arrays, persistent handles, and nested records are now
also proven through the real downstream descriptor. `use hello`, compilation,
dynamic loading and compiled-provider startup remain tracked contracts.

## What is here

```text
crates/mlpl-extension-abi/      Versioned C-compatible ABI and validation
crates/mlpl-extension-loader/   Package resolver, dynamic loader, and registry
crates/mlpl-extension-sdk/      Safe author-facing SDK scaffold
crates/mlpl-native3d-scene/      Generic line/point scenes and headless planning
lib/native3d/                    Reusable MLPL camera, picking, geometry, app loop
demos/wireframe-cube/            MLPL-owned bulk-array cube scene
demos/tic-tac-toe/               MLPL rules, minimax, and generic line scene
demos/life-plane/                 MLPL finite-grid Life model and presets
extensions/hello/               Rust cdylib, package manifest, and MLPL facade
extensions/boundary-probe/      Public-SDK array/handle/record host probe
extensions/native3d/            Generic headless viewer and bulk line provider
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

Open the interactive native cube:

```sh
just cube-3d
```

Open the playable native tic-tac-toe game:

```sh
just tic-tac-toe
```

Open the editable native Life plane:

```sh
just life-3d
```

Open Life on a native 3D torus with wrap-around in both grid axes:

```sh
just life-torus
```

Measure bounded Model Atlas range scanning against growing sparse files:

```sh
just model-atlas-memory-evidence
```

Validate the derived Safetensors/GGUF tensor-city handoff:

```sh
just model-atlas-contract
```

Open the interactive native Model Atlas tensor city:

```sh
just model-atlas
```

Choose and inspect an actual local Safetensors model without reading its
tensor payload:

```sh
just model-atlas-file
```

The picker searches `../demo-ml-utils/models` by default. Use an absolute
override such as `MODEL_ROOT=/absolute/model/directory just model-atlas-file`.
Use Up/Down and Enter to choose a file, then M to return to the menu. See
[Model Atlas real files](docs/model-atlas-real-files.md) for bounds, ownership,
format support, and the filesystem containment contract.

Explore a bounded sample of real Safetensors or GGUF tensor values:

```sh
just weight-distribution
just microscope-web
```

The default picker searches `../demo-ml-utils/models`; set an absolute
`MODEL_ROOT` to inspect a different confined tree. Choose a model and tensor
with Up/Down and Enter. The histogram uses value units on X and logarithmic
sample counts on Y; drag to orbit, Shift-drag or middle-drag to pan, use the
wheel to zoom, Left/Backspace to return to tensors, and M to return to models.
Only explicitly supported integer and Q8_0 decoders are enabled. See the
[weight-distribution guide](docs/native3d-weight-distribution.md) and
[blocker matrix](docs/weight-distribution-blockers.md).

The bundled, derived fixture renders tensor metadata as stable-ID wireframe
buildings grouped into Safetensors and GGUF districts. Building height uses a
visible logarithmic stored-byte scale, so one large tensor cannot flatten the
rest of the view. Click a building to inspect its name and heuristic role,
drag to orbit, Shift-drag or middle-drag to pan, and use the wheel to zoom.
A selects all tensors; S and G toggle their Safetensors or GGUF filter; L
switches independently between footprint and height detail; R resets the view.
The visible source panel and legend document provenance, architecture metadata,
colors, scales, current filter/LOD state, and controls. See the
[Model Atlas native city](docs/model-atlas-native-city.md) for the data,
ownership, bounded-detail, and platform contracts.

Click toggles a cell and Control-left-drag paints live cells. Plain left-drag
orbits/tilts, Shift-left-drag or middle-drag pans, and the wheel zooms. Space
runs/pauses, N steps, C clears, plus/minus changes speed, and B/H/I/T/G/U/R
select block, beehive, blinker, toad, glider, Gosper gun, and seeded random.
The same complete legend is visible inside the native window.
The torus uses the same controls and presets; cells crossing either edge
continue at the opposite edge, and clicking or painting follows the curved
surface.

Click an empty square to move. Left-drag orbits/tilts, the wheel zooms, and
Shift-left-drag or middle-drag pans; crossing the four-pixel drag threshold
suppresses mark placement. X/O chooses the human mark, 1/2 chooses first or
second, R restarts with those choices, and Escape closes. The board rules,
perfect-play strategy, picking, choices, turns, hover, and scene arrays are
MLPL-owned; the shared Rust host only normalizes input and renders generic
lines.

The window keeps winit/wgpu on the main thread and runs sw-MLPL on a worker.
`controls.mlpl` receives generic key/resize events and sends retained patches
or view-only commands back to the renderer after initialization. Use W/S for width, arrows for height, A/D
for length, +/- for signed speed, Space for pause, C for color, brackets for
thickness, R for reset, and Escape to close. Left-drag orbits and tilts, the
wheel zooms, and Shift-left-drag or middle-drag pans; these mappings and camera
state are implemented in MLPL and shown in the window legend.

Run focused Rust or MLPL tests:

```sh
just rust-tests
just tests
just list-tests
```

The native demos include an editable Conway's Life plane. Its MLPL layer
provides dead finite boundaries, whole-array B3/S23 evolution, owned cell
updates, deterministic replacement presets, mouse cell editing, animation and
speed controls, shared orbit/pan/zoom, and the complete visible control legend.
connects to the existing native host through generic stable-ID bulk line arrays.

Run the mandatory pre-commit gate:

```sh
just check
```

The complete gate checks repository layout, `.gitignore`, tracked files,
public/private namespaces, canonical MLPL formatting, MLPL module comments and
function docstrings, Rust formatting, compilation, clippy, all Rust tests,
native mlplunit tests, and whitespace. Set `MLPL_FMT` to an absolute formatter
path when the adjacent `../sw-mlpl` checkout is unavailable. The intentional panic test may
print its panic-hook message; the test verifies that the panic is converted to
`ExtensionPanicked` before it can unwind across the C ABI.

To run only the dynamic hello acceptance tests:

```sh
cargo build -p mlpl-extension-hello
cargo test -p mlpl-extension-loader --test hello_registration
cargo test -p mlpl-extension-loader --test manifest_resolution
```

## Current status

The delivered repository proves:

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
- a bounded renderer-neutral point-scene contract for `[N,3]` positions and
  parallel size/color/opacity/stable-ID arrays, with deterministic upload
  planning and explicit owned-byte accounting;
- deterministic headless point projection, viewport/near-plane culling,
  far-to-near alpha ordering, stable-ID picking ties, and raster evidence;
- a native wgpu/winit circular point-sprite pipeline with full stable IDs and an
  opt-in `just point-cloud-smoke` fixture on graphical macOS/Linux sessions;
- atomic retained point replacements/patches and precision-safe stable-ID
  selection events, with application meaning reserved for MLPL;
- a deterministic `just point-cloud` MLPL teaching app with selection, camera,
  patches, lifecycle, and a headless real-worker acceptance test;
- a bounded `just point-cloud-acceptance` release probe and evidence matrix that
  separate CPU byte/timing observations from unmeasured GPU/Linux behavior;
- a deterministic headless transform, perspective projection, clipping, and
  thick-line raster pipeline with portable PPM evidence.
- a real headless `_native3d` provider with typed viewer lifecycle, bulk line
  arrays, state/size records, and explicit MLPL-supplied render state.
- an MLPL-owned control reducer for dimensions, signed speed, pause/reset,
  palette, thickness, resize/close events, and deterministic bulk updates.
- bounded/coalesced input, camera/picking, single-flight animation frames, and
  atomic retained-scene patches across every interactive native demo;
- native tic-tac-toe, Life plane/torus, Model Atlas, disk usage, audio spectrum,
  and weight-distribution applications with MLPL-owned semantics;
- an offline-first Rust/Yew microscope over schema-validated, hash-pinned MLPL
  recordings.

The opt-in wgpu/winit window is connected to the MLPL reducer through sw-MLPL's
parked-main Port contract. Only owned event and scene values cross between the
main-thread UI and worker interpreter. Dynamic loading by sw-MLPL, real
unload/hot reload, facades, and compiled-provider startup remain future work.

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
- [MLPL wireframe-cube controls](docs/wireframe-cube-controls.md) — normalized
  event records, reducer behavior, bulk updates, and final event-loop seam.
- [Headless wireframe renderer](docs/headless-wireframe-renderer.md) — pure
  transform/projection/clipping pipeline, deterministic evidence, and the
  MLPL-owned interactive boundary.
- [Headless native3d provider](docs/headless-native3d-provider.md) — public
  primitives, bulk-array contract, lifecycle evidence, and deliberate scope.
- [Native window](docs/native-window.md) — opt-in cube command, wgpu/winit
  architecture, macOS/Linux handling, and live-interaction blocker.
- [Native3D interaction contract](docs/native3d-interaction-contract.md) —
  bounded pointer/frame events, orbit-camera coordinates, and pick rays.
- [Native3D MLPL library](docs/native3d-mlpl-library.md) — reusable camera,
  picking, grid/line, callback, and application-lifecycle helpers.
- [MLPL tic-tac-toe model](docs/tic-tac-toe-model.md) — validated board,
  legal moves, outcomes, player setup, and deterministic minimax.
- [Native tic-tac-toe acceptance](docs/tic-tac-toe-acceptance.md) — playable
  behavior, regressions, ownership, portability, and remaining limits.
- [MLPL Life model](docs/life-model.md) — finite boundary policy, vectorized
  evolution, owned grids, deterministic presets, and upstream comparison.
- [MLPL Life controls](docs/life-controls.md) — editing gestures, animation,
  presets, camera arbitration, visible bindings, and native ownership split.
- [Native Life acceptance](docs/life-acceptance.md) — responsiveness fix,
  retained view/geometry diffs, bounds, and platform evidence.
- [Toroidal Life](docs/life-torus.md) — two-axis wrap topology, curved mapping,
  surface picking, controls, ownership, and current performance bounds.
- [Retained scene patches](docs/retained-scene-patches.md) — stable-ID atomic
  line diffs, revision/resync behavior, bounds, ownership, and measured scope.
- [Native3D point-cloud contract](docs/native3d-point-cloud.md) — bulk point
  shapes, attributes, stable IDs, budgets, rendering, retained updates, and
  selection delivery.
- [Native3D point-cloud demo](docs/native3d-point-cloud-demo.md) — deterministic
  MLPL-owned arrays, controls, selection styling, and host responsibility split.
- [Native3D point-cloud acceptance](docs/native3d-point-cloud-acceptance.md) —
  release/package evidence, measured CPU bounds, portability, and limitations.
- [Native 3D disk-usage explorer](docs/native3d-disk-usage.md) — bounded
  metadata-only scanning, cached MLPL navigation, color semantics, and
  retained shadow-scene diffs.
- [Native 3D audio spectrum player](docs/native3d-audio-spectrum.md) — bounded
  MP3/Ogg-Vorbis decoding, MLPL-owned stereo radial analysis, synchronized
  default-device audio, and retained scene diffs.
- [Bounded Model Atlas scanning](docs/model-atlas-bounded-scan.md) — range-read
  passes, compact adapter columns, selected detail/cache bounds, and RSS data.
- [Model Atlas interchange](docs/model-atlas-interchange.md) — versioned tagged
  transport, derived cross-format fixture, provenance, layout, and ownership.
- [Model Atlas native city](docs/model-atlas-native-city.md) — interactive
  tensor buildings, filtering, selection, camera controls, and bounded LOD.
- [Model Atlas real files](docs/model-atlas-real-files.md) — confined model
  picker, bounded Safetensors catalog reads, ownership, and current limits.
- [Model Atlas architecture](docs/model-atlas-architecture.md) — authoritative
  GGUF metadata versus explicitly qualified tensor-name role inference.
- [Model Atlas detail views](docs/model-atlas-detail.md) — bounded statistics,
  histogram/surface and Q8-error fixtures, retained patches, and limitations.
- [Wireframe cube acceptance](docs/wireframe-cube-acceptance.md) — evidence
  matrix and deliberately narrow PoC claims.
- [sw-MLPL blockers](docs/sw-mlpl-blockers.md) — exact handles, arrays, events,
  viewer-call, and compiler requirements for MLPL-owned interaction.
- [Compiled 3D app blocker and library split](docs/compile-3d-app-blocked.md) —
  concise MLPL wrapper design and exact upstream compile/package requirements.
- [Extension packages](docs/extension-packages.md) — manifest, platform,
  path-security, and namespace contracts.
- [Development and testing](docs/development.md) — tool resolution, TDD, and
  repository commands.
- [Implementation plan](docs/plan.md) — recommended architecture, capability
  gates, and demo order.
- [Saga status and candidates](docs/sagas.md) — delivered sagas, partial future
  scope, and the recommended point-cloud/embedding direction.
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
