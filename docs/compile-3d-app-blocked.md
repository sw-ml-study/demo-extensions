# Compiled 3D App: Blocker and MLPL Library Split

Status date: 2026-08-15

The interactive cube is complete in interpreted mode, but the same MLPL source
cannot yet be emitted as a self-contained native executable. The blocker is not
cube geometry or GPU support. It is the missing sw-MLPL compiler/runtime path
for starting a native provider, parking the platform UI loop on the main thread,
and running compiled MLPL against the same Port and extension contracts.

An MLPL library should wrap this generic machinery. That would make the demo
shorter without moving cube semantics into Rust.

## Recommended responsibility split

### Rust extension and host

The native layer remains application-neutral. It owns:

- winit window creation and the macOS/Linux main-thread event loop;
- wgpu adapter, surface, pipeline, buffer, render, present, and resize work;
- conversion of platform input into bounded normalized event records;
- typed generational handles and native resource lifetimes;
- validation and copying at the C ABI boundary;
- transport of owned commands and events through a bounded Port.

It must not know that `W` changes cube width, which palette follows `C`, or how
cube vertices are generated.

### Reusable MLPL library

A proposed `native3d/app.mlpl` library should own reusable language-level
policy and protocol details:

- construct and validate the generic `set_scene` command;
- turn one color/thickness into parallel arrays when uniform styling is wanted;
- allocate stable line IDs and validate `[N,3]`, `[M,2]`, `[M,4]`, and `[M]`
  relationships;
- submit the initial scene, register key/resize/close handlers, and call
  `run(port, state)`;
- provide `clamp`, normalized-key predicates, palette helpers, and common
  resize/close state updates;
- preserve errors rather than silently dropping invalid scenes or Port sends.

The library should be ordinary inspectable MLPL, not privileged host code. Its
public surface can stay small:

```mlpl
# native3d/app.mlpl — illustrative API, not implemented yet
def n3d:lines(positions,edges,style) { ... }
def n3d:uniform_style(edge_count,color,thickness) { ... }
def n3d:run_app(initial,reduce,render,help) { ... }
```

`run_app` receives function references and owns only the generic event/command
wiring. `reduce(state,event)` and `render(state)` remain application callbacks.
This is preferable to a cube-specific `open_cube` primitive because other MLPL
apps can reuse the same library for plots, point clouds, graphs, and editors.

### Concise application

With that library, one self-contained cube application could reasonably be
about 25–40 lines instead of the current 87 substantive lines across three
files. Its shape would be:

```mlpl
include "native3d/app.mlpl";

def app:initial() {
  {width:2,height:2,length:2,speed:0.6,paused:0,color:[0.2,0.8,1,1],thickness:2}
}

def app:reduce(state,event) {
  # Cube-specific mapping: W/S dimensions, Space pause, C palette, R reset.
  ...
}

def app:render(state) {
  positions=app:cube_positions(state.width,state.height,state.length);
  edges=[0,1,1,2,2,3,3,0,4,5,5,6,6,7,7,4,0,4,1,5,2,6,3,7];
  style=n3d:uniform_style(12,state.color,state.thickness);
  n3d:lines(reshape(positions,[8,3]),reshape(edges,[12,2]),style)
}

n3d:run_app(app:initial(),:app:reduce,:app:render,
  "W/S WIDTH  SPACE PAUSE  C COLOR  R RESET  ESC CLOSE")
```

This can also be physically one `.mlpl` file today by inlining the current
includes. The proposed library improves conceptual size; merely concatenating
files does not.

## What sw-MLPL must add for native compilation

The compiler needs parity with every host facility used by the interpreted
applet:

1. **Compiled provider registration.** Generated programs need a supported
   startup hook that registers a statically linked C-descriptor provider before
   evaluating MLPL. Registration, namespace resolution, diagnostics, and
   deactivation must match interpreted execution.
2. **Parked-main launch inversion.** The generated executable must enter winit
   on the process main thread and execute compiled MLPL on a worker. This must
   be a compiler/runtime contract, not cube-specific generated Rust.
3. **Compiled Port parity.** `port_send`, handler `on`/`off`, `run`, bounded
   polling/delivery, disconnect, cancellation, and shutdown must work in
   compiled code with the same ordering and error behavior.
4. **Value-boundary parity.** Dense arrays, nested records, strings, errors,
   and typed generational handles must cross compiled provider calls with the
   same shape, ownership, copy, and lifetime rules as the interpreter.
5. **Module/include packaging.** The compiler must resolve the application and
   `native3d/app.mlpl`, record those inputs, and embed or install them without
   depending on this repository's source-tree paths at runtime.
6. **Native artifact linkage.** The build must link the MLPL runtime, provider,
   winit/wgpu dependencies, and platform graphics/window libraries into (or
   alongside) the produced application without using Rust's unstable ABI.
7. **Target-aware packaging.** macOS and Linux are separate native builds. The
   MLPL source and public library API stay identical, while Cargo/its eventual
   packaging driver selects Metal-related macOS integration or an available
   Linux wgpu backend and the corresponding winit system dependencies. This is
   target selection, not conditional application logic.
8. **Deterministic failure and teardown.** Startup failure, worker panic,
   provider error, window close, Port disconnect, and renderer failure must
   join/finalize cleanly without stale handles or orphaned threads.

The first acceptable implementation may use static provider linkage. Dynamic
discovery and true `dlclose` are not prerequisites for compiling this demo.

## Required upstream acceptance

The capability is unblocked only when sw-MLPL can demonstrate all of these:

- compile the same concise cube source without a source rewrite;
- launch a real window with winit on main and compiled MLPL on a worker;
- receive keys and resize/close events through a bounded Port;
- pause and resume via MLPL state and submit bulk line arrays successfully;
- reject malformed arrays and stale/wrong-type handles consistently;
- close repeatedly without leaked threads, live handles, or a hung process;
- build on both macOS and Linux CI/toolchains, with an opt-in graphical smoke
  test on an actual host for each platform;
- keep equivalent headless tests for environments without a display.

## Downstream work after upstream parity

Once those compiler contracts ship, this repository can implement and test
`native3d/app.mlpl`, migrate the cube to the concise API, add an interpreted vs
compiled behavior test, and package `just build-3d-cube` (name provisional).
Until then, claiming a native compiled cube would confuse the current Rust host
that embeds an interpreter worker with MLPL compiler output.

## Non-goals

- Moving cube controls or geometry into Rust.
- Introducing a game engine, browser, HTML, JavaScript, or WASM.
- Claiming zero-copy arrays before ownership tests and measurements prove it.
- Requiring dynamic loading or true library unload for the first compiled app.
- Hiding the public C ABI behind a repository-only shortcut unavailable to a
  third-party extension.

Current interpreted evidence is documented in
[`native-window.md`](native-window.md), [`wireframe-cube-controls.md`](wireframe-cube-controls.md),
and [`wireframe-cube-acceptance.md`](wireframe-cube-acceptance.md). The broader
host contract remains in [`upstream-contract.md`](upstream-contract.md).
