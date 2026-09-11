# How the demos load native code

This repository currently demonstrates three different integration shapes. They
should not be conflated:

1. **Dynamic MLPL loading:** TodoMVC starts the normal `mlpl-repl` executable,
   and its MLPL source calls `load_extension` with explicit `.dylib` or `.so`
   paths.
2. **A Rust application hosting MLPL:** the interactive 3D applications start
   `mlpl-native3d-window`. That executable embeds the MLPL evaluator, injects a
   `Port`, and owns winit/wgpu, filesystem, and audio services. It does not
   dynamically load the `native3d` cdylib.
3. **A statically linked provider host:** the HTTP-client launcher builds a tiny
   Rust host that links the HTTP provider, registers its C descriptor, and then
   evaluates the `.mlpl` demo.

The deterministic TodoMVC preview and model/framework tests need no native
provider. The Yew microscope is a Rust/Wasm application rather than an MLPL
extension consumer.

## Shared-library names and locations

Each extension crate declares `crate-type = ["cdylib", "rlib"]`. A debug Cargo
build places the loadable file directly under `target/debug/`:

| Extension crate | macOS artifact | Linux artifact | Registered namespace |
|---|---|---|---|
| `mlpl-extension-hello` | `libmlpl_extension_hello.dylib` | `libmlpl_extension_hello.so` | `_hello` |
| `mlpl-extension-http-client` | `libmlpl_extension_http_client.dylib` | `libmlpl_extension_http_client.so` | `_http` |
| `mlpl-extension-http-server` | `libmlpl_extension_http_server.dylib` | `libmlpl_extension_http_server.so` | `_web` |
| `mlpl-extension-sqlite` | `libmlpl_extension_sqlite.dylib` | `libmlpl_extension_sqlite.so` | `_sqlite` |
| `mlpl-extension-native3d` | `libmlpl_extension_native3d.dylib` | `libmlpl_extension_native3d.so` | `_native3d` |
| `mlpl-extension-boundary-probe` | `libmlpl_extension_boundary_probe.dylib` | `libmlpl_extension_boundary_probe.so` | `_boundary` |
| `mlpl-extension-canvas` | `libmlpl_extension_canvas.dylib` | `libmlpl_extension_canvas.so` | `_canvas` |

Release builds use the same filenames beneath `target/release/`. The
`extensions/*/extension.toml` files describe a future distributable package
layout under `native/<target-triple>/`; Cargo does not copy build products into
those directories automatically. Every library exports the versioned C entry
symbol `sw_mlpl_extension_v1`. Loading native code is a trust decision: only
load artifacts you built or otherwise trust.

The interpreter accepts either loading form:

```mlpl
# Explicit path: the form used by TodoMVC.
load_extension("/absolute/path/libmlpl_extension_sqlite.dylib")

# Logical name: searched as libmlpl_extension_sqlite.dylib/.so.
load_extension("mlpl_extension_sqlite")
```

The second form searches the colon-separated directories in
`MLPL_EXTENSION_PATH`. This repository's launchers intentionally use absolute
paths instead, so they neither mutate nor depend on the caller's environment.
`load_extension` returns the descriptor's namespace on success and an MLPL
`err(...)` value for a missing or invalid library. V1 retains a successfully
loaded library for the process lifetime; it does not promise true `dlclose`.

## TodoMVC: two dynamically loaded libraries

`just todomvc-server` delegates to `scripts/run-todomvc-server`, which performs
the complete setup:

1. Validate `TODOMVC_DATA_DIR`, `TODOMVC_DB_NAME`, and `TODOMVC_PORT`; create
   the confined data directory.
2. Run an offline debug build of `mlpl-extension-http-server` and
   `mlpl-extension-sqlite`.
3. Select `MLPL` when it is an absolute executable path, otherwise a suitable
   `mlpl-repl` from PATH or the adjacent `../sw-mlpl/target/{release,debug}`.
4. Derive the platform suffix and pass both absolute library paths, the data
   root, database filename, and port after `--` as MLPL arguments.
5. Execute `demos/todomvc/server.mlpl`. Its first native actions are
   `load_extension(web_library)` and `load_extension(sqlite_library)`; it then
   calls `_sqlite:*` and `_web:*` functions registered by those descriptors.

The effective command shape on macOS is:

```sh
cargo build --offline -p mlpl-extension-http-server -p mlpl-extension-sqlite
../sw-mlpl/target/release/mlpl-repl --source-dir "$PWD" \
  -f "$PWD/demos/todomvc/server.mlpl" -- \
  "$PWD/target/debug/libmlpl_extension_http_server.dylib" \
  "$PWD/target/debug/libmlpl_extension_sqlite.dylib" \
  "$PWD/var/todomvc" todos.sqlite3 3000
```

Linux substitutes `.so`. `just todomvc-reset` repeats the SQLite build/load
path and runs `demos/todomvc/reset.mlpl`. No separate database initializer is
needed: `server.mlpl` executes `CREATE TABLE IF NOT EXISTS` after opening the
connection. `just todomvc` is different—it runs `demos/todomvc/demo.mlpl` and
only prints deterministic HTML, so it needs neither extension nor database.

A plain interpreter invocation of `server.mlpl` without its path arguments
fails while reading `args()`. Supplying arguments but omitting the builds, or
calling `_web:*`/`_sqlite:*` without successful `load_extension` calls, fails
because those private native functions are not registered.

## Interactive graphics: MLPL inside a custom Rust host

The small `just array-canvas` example is the exception: it follows TodoMVC's
dynamic-loading path and calls one blocking canvas function from stock
`mlpl-repl`. See [Dynamic Array Canvas](canvas-extension.md). The richer demos
below require the custom host because they exchange ongoing input and frames.

The graphics commands all build and run the release binary
`target/release/mlpl-native3d-window`:

| Command | Host selector | MLPL application source | Extra host authority |
|---|---|---|---|
| `just cube-3d` | default | `demos/wireframe-cube/*` | window/GPU/input |
| `just tic-tac-toe` | `--tic-tac-toe` | `demos/tic-tac-toe/*` | window/GPU/input |
| `just life-3d` | `--life` | `demos/life-plane/*` | window/GPU/input/ticks |
| `just life-torus` | `--life-torus` | Life plane plus `demos/life-torus/*` | window/GPU/input/ticks |
| `just point-cloud` | `--point-cloud` | `demos/point-cloud/*` | window/GPU/input/picking |
| `just model-atlas` | `--model-atlas` | `demos/model-atlas/*` plus fixtures | window/GPU/input |
| `just model-atlas-file` | `--model-atlas-file ROOT` | `demos/model-atlas-file/*` plus bounded format modules | confined model root |
| `just disk-usage` | `--disk-usage ROOT` | `demos/disk-usage/*` | bounded read-only snapshot/root |
| `just audio-spectrum` | `--audio-spectrum ROOT` | `demos/audio-spectrum/*` | confined audio discovery/decode/output |
| `just weight-distribution` | `--weight-distribution ROOT` | `demos/weight-distribution/*` plus bounded format modules | confined model root |
| `just point-cloud-smoke` | `--point-scene FILE` | default cube applet plus a JSON point fixture | explicit bounded fixture read |
| `just box-scene-smoke` | `--box-scene FILE --selected-box ID` | default cube applet plus a JSON box fixture | bounded fixture read/GPU depth |

The corresponding `scripts/run-*` file changes to the repository root and
runs `cargo run --release -p mlpl-native3d-window` with the selector above.
`MODEL_ROOT`, `DISK_USAGE_ROOT`, and `AUDIO_SPECTRUM_ROOT` configure the rooted
demos; their scripts require absolute paths.

At compile time, `crates/mlpl-native3d-window/src/live.rs` uses `include_str!`
to embed and assemble the tracked MLPL modules. At runtime, the host creates an
MLPL `Environment`, registers two bounded Rust channels as a typed Port handle,
and inserts that handle under the global name `port`. MLPL owns state,
reducers, geometry, controls, and scene commands. Calls such as
`port_send(port, {op: "set_scene", ...})` cross to the Rust event loop; Rust
normalizes input and sends records back for MLPL handlers registered with
`on(port, ...)`. The host—not an MLPL-loaded library—owns winit/wgpu and the
main-thread event loop.

The generic `_native3d` cdylib now has a normal package manifest and a small
public MLPL helper module under `extensions/native3d/`. A stock `mlpl-repl` can load its
platform library with `load_extension` (or resolve it through
`MLPL_EXTENSION_PATH`) and call the bulk headless resource API: `set_lines`,
`set_boxes`, `set_view`, `pick_box`, `set_selection`, `render`, and `close`.
Static and actual dynamically loaded provider tests execute the same versioned
C descriptor and validation. Calls taking a viewer currently use the registered
`_native3d:*` names directly because sw-MLPL cannot yet bind a native handle to
a user-function parameter. Arrays are copied into provider-owned storage for
the viewer lifetime; handles are typed and generational, and stale handles are
rejected.

This API deliberately does not claim that stock `mlpl-repl` can open and drive
the winit/wgpu window. Window creation and event delivery have main-thread and
event-loop constraints, and the V1 callback-free ABI has no host Port injection
contract for them. Consequently, these interactive commands do **not** consult
`MLPL_EXTENSION_PATH`, do not execute `load_extension`, and do not use the
separately built `libmlpl_extension_native3d.*`. That cdylib proves the public
ABI's generic scene/handle provider, including boxes, orthographic views,
picking, and selection state, but the shipped windowed
applications require the custom host until the stock CLI can configure a UI
main loop and inject a Port. Running a `live-applet.mlpl` directly with plain
`mlpl-repl` fails because `port` has not been registered/injected (and a stock
CLI does not own the required native event loop). Pure model, scene, and
controller modules remain independently testable with mlplunit.

## HTTP client: statically linked descriptor

`just http-client [URL]` runs `scripts/run-http-client`. It sets an isolated
Cargo target directory at `target/upstream-host`, builds the
`http_client_demo` binary offline, and starts it with the URL. The host source
is `tests/upstream-host/src/bin/http_client_demo.rs`.

That Rust binary links `mlpl-extension-http-client` as an `rlib`, calls its
`static_entry()`, registers the returned V1 C descriptor through sw-MLPL's
public adapter, reads `demos/http-client/get.mlpl`, places the URL in MLPL's
argument list, and evaluates it. The MLPL source then calls `_http:get(url)`.
There is no `dlopen`, library-path variable, or `load_extension` in this demo;
static and dynamic providers share descriptor validation and dispatch.

Running `get.mlpl` in an ordinary extension-free interpreter reaches an
undefined `_http:get` function. The `http-client` cdylib can be built and
loaded explicitly, but the provided launcher deliberately exercises the
static-provider host acceptance path.

## Other demos and proofs

| Demo or command | Native loading behavior |
|---|---|
| `just todomvc` | Pure MLPL deterministic HTML preview; no native library. |
| `demos/experiment-dashboard/crud.mlpl` | Pure MLPL framework/domain proof used by mlplunit; no server process or native library. |
| `just model-atlas-contract` and memory evidence | Validation/measurement scripts, not interactive extension-loading demos. |
| `just microscope-web` | Rust/Yew compiled to Wasm and served by Trunk; no MLPL native extension. Optional live SSE uses a separately started `mlpl-serve`. |
| `mlpl-extension-hello` tests | `hello_registration` uses this repository's `libloading`-based test host to open the actual cdylib. The public facade/package manifest is a teaching proof, not a current demo launcher. |
| `mlpl-extension-native3d` tests | Load or statically register the generic headless provider to prove arrays, handles, lifecycle, and malformed-input behavior; the windowed demos use the custom host described above. |

## Manual diagnostics

To see the files Cargo produced:

```sh
cargo build -p mlpl-extension-http-server -p mlpl-extension-sqlite
find target/debug -maxdepth 1 -type f \
  \( -name 'libmlpl_extension_*.dylib' -o -name 'libmlpl_extension_*.so' \)
```

To prove that TodoMVC is genuinely loading native libraries, temporarily give
`server.mlpl` a nonexistent first library path: `load_extension` returns an
error and the following `_web:listen` is unavailable. Do not test this against
your persistent server process; the repository's provider and live acceptance
tests cover malformed descriptors and missing artifacts without touching the
normal database.

Authoritative implementation pointers are
`scripts/run-todomvc-server`, `demos/todomvc/server.mlpl`,
`tests/upstream-host/src/bin/http_client_demo.rs`,
`crates/mlpl-native3d-window/src/main.rs`, and
`crates/mlpl-native3d-window/src/live.rs`. Package-layout rules are in
[Extension packages](extension-packages.md), while ABI ownership and lifecycle
are covered by [Hello dynamic extension](hello-extension.md) and
[Upstream sw-MLPL contract](upstream-contract.md).
