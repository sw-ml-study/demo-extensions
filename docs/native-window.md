# Native wgpu/winit Window

The opt-in `cube-3d` recipe runs a real interactive desktop app without a
browser or game engine. Its workflow is intentionally visible:

1. winit/wgpu owns the process main thread.
2. sw-MLPL evaluates the scene, controls, and live applet on a worker.
3. The UI sends owned normalized key, pointer, wheel, frame, resize, and close
   records over a Port.
4. MLPL reduces state and returns owned bulk scene and camera commands.
5. The shared pipeline plans each frame and wgpu draws thick-line triangles.

Run it from a graphical macOS or Linux session:

```sh
just cube-3d
```

Controls: W/S width, arrows height, A/D length, +/- signed speed, Space pause,
C color, brackets thickness, R reset, and Escape/native close to exit. The
command is opt-in and not part of `just check`; the same two-thread protocol is
covered by a headless automated host test. The view renders this help legend;
its title shows the current MLPL revision, dimensions, and signed speed so each
accepted key has immediate visible feedback.

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

## Architecture and remaining scope

Rust normalizes physical keys but contains no cube mapping. The mapping and all
state transitions are in `controls.mlpl`; the host only applies generic scene
commands and integrates the MLPL-provided signed speed.

The host also normalizes physical-pixel cursor coordinates, buttons, modifiers,
wheel units, and coalescible frame timing. Pending motion replaces older motion,
wheel deltas accumulate, frame timing replaces older pending timing, and button
transitions remain ordered. A full queue is flushed before another discrete
event is accepted. None of these generic events has cube meaning in Rust.

An optional scene-command camera record contains target `[3]`, yaw, pitch,
distance, vertical field of view, and near plane. Missing camera state uses the
old default, so the keyboard cube remains compatible. The MLPL camera library
and cube mouse mappings are intentionally the next two saga steps.

The target public extension API remains generic:

- `open` returns a typed viewer handle;
- `poll_events` returns a bounded batch of generic key/pointer/wheel/frame,
  resize, and close events;
- `set_lines` updates bulk positions, edges, colors, thicknesses, and IDs;
- `render`/`present` accepts MLPL-owned time or transform state;
- `drawable_size`, monotonic time, and `close` complete the lifecycle.

The local interpreted event loop is complete. sw-MLPL's `run` dispatches one
owned event at a time on the worker (a stricter per-dispatch bound than a batch)
while the UI remains responsive. Compiled-provider parity, dynamic loading,
and true unload remain separate future work.
