# Native wgpu/winit Window

The opt-in `cube-3d` recipe proves a real desktop window without a browser or
game engine. Its workflow is intentionally visible:

1. sw-MLPL evaluates `demos/wireframe-cube/default-scene.mlpl`.
2. The script generates deterministic generic line-scene JSON in a temporary file.
3. `mlpl-native3d-window` validates that scene and opens a winit window.
4. The shared headless pipeline plans each rotated frame; wgpu draws its thick-line triangles.

Run it from a graphical macOS or Linux session:

```sh
just cube-3d
```

Press Escape or use the native close control to exit. The command is opt-in and
is not part of `just check`; automated acceptance remains headless. The first
release build downloads and compiles the wgpu/winit dependency graph.

`just` recipe identifiers cannot begin with a digit, so `just 3d-cube` is not a
valid recipe spelling in the installed tool. `just cube-3d` is the documented
equivalent.

## macOS and Linux

The application has one renderer and one WGSL shader. winit selects Cocoa on
macOS and Wayland or X11 on Linux through its target-specific crate code. wgpu
selects Metal on macOS and a supported native backend—normally Vulkan—on Linux.
Surface formats are queried at runtime rather than hard-coded.

There are no separate Mac and Linux application implementations. Conditional
compilation remains inside winit/wgpu unless a future platform integration
requires a small isolated adapter. Linux needs a working graphical session and
GPU driver; headless servers can run all logic tests but cannot perform the
window smoke check.

## PoC boundary and current blocker

The smoke executable advances rotation using the MLPL-provided speed so the
native renderer can be inspected now. Escape and native close are lifecycle
controls, not cube behavior. It deliberately does not implement Rust-owned key
bindings for cube dimensions, speed, colors, or thickness.

The target public extension API remains generic:

- `open` returns a typed viewer handle;
- `poll_events` returns a bounded batch of generic input/resize/close events;
- `set_lines` updates bulk positions, edges, colors, thicknesses, and IDs;
- `render`/`present` accepts MLPL-owned time or transform state;
- `drawable_size`, monotonic time, and `close` complete the lifecycle.

MLPL now owns and tests the reducer that maps normalized event records to cube
state and deterministic bulk scene updates. Persistent handles, bulk extension
arrays, and structured records are proven through the actual interpreter.
Native event polling and host event-loop integration remain upstream work, as
does compiled-provider parity, so the end-to-end interactive loop is still
blocked. The PoC window does not disguise a Rust control loop as completion of
that contract.
