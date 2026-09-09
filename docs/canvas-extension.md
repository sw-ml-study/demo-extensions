# Dynamic Array Canvas

The Array Canvas is the smallest graphical proof with the same launch shape as
TodoMVC: a stock `mlpl-repl` process evaluates a tracked program, that program
calls `load_extension` with an absolute shared-library path, and later MLPL code
invokes functions registered by the library.

Run it with:

```sh
just array-canvas
```

`scripts/run-array-canvas` builds `mlpl-extension-canvas` offline, selects the
normal interpreter, chooses `.dylib` on macOS or `.so` on Linux, and supplies
the absolute debug artifact path to `demos/array-canvas/spiral.mlpl`. The MLPL
program loads that path before calling `u:canvas_show`. Closing the native
window returns a `{points, lines, closed}` record and lets the interpreter exit.

The responsibility split is intentionally visible:

- `model.mlpl` uses `range`, `sin`, `cos`, whole-array arithmetic, and a bounded
  assembly loop to produce `[720,2]` normalized points, `[719,4]` RGBA colors,
  and `[719]` pixel thicknesses.
- `module.mlpl` is the public MLPL facade over `_canvas.validate` and
  `_canvas.show`.
- the Rust provider validates/copies title, dimensions, dtype, rank, shape,
  finiteness, coordinate/color ranges, thickness, and a 100,000-point cap.
- `window.rs` owns only generic winit/wgpu window, resize, redraw, GPU resource,
  and close behavior. It contains no spiral or MLPL application semantics.

`show` is deliberately blocking. It creates and runs winit on the interpreter
process's main thread and returns only after close. V1 has no callbacks,
background MLPL thread, injected Port, interaction API, animation protocol, or
second-window promise. Those features require a separate lifecycle design.

The mandatory headless acceptance runs `scripts/check-array-canvas`. It loads
the actual cdylib through `load_extension`, calls `_canvas.validate` through the
real interpreter, and separately proves the same call fails when no extension
was loaded. Rust tests cover malformed bulk arrays and descriptor registration metadata;
mlplunit proves the spiral shapes and bounds without requiring a display. The
interactive `just array-canvas` smoke remains opt-in.

Build artifacts are:

```text
target/debug/libmlpl_extension_canvas.dylib  # macOS
target/debug/libmlpl_extension_canvas.so     # Linux
```

The package manifest records the corresponding target-specific distribution
paths under `extensions/canvas/native/<target>/`; Cargo does not populate those
package directories automatically. See [How demos load native code](demo-extension-loading.md)
for comparison with the Port-injecting graphics host and statically registered
HTTP client.
