# Native Toroidal Life

`just life-torus` opens Conway's Life on a native 3D donut. The 20×40 grid
wraps in both axes: a neighbor beyond the last row or column is read from the
first row or column. This topology is uniform at every cell and avoids the
special pole policy that a future sphere demonstration must define explicitly.

## Controls

- Click toggles the nearest surface cell; Control-left-drag paints live cells.
- Left-drag orbits and tilts. Shift-left-drag or middle-drag pans. The wheel
  zooms.
- Space runs or pauses, N advances one generation, C clears, and plus/minus
  changes speed.
- B, H, I, T, G, U, and R select block, beehive, blinker, toad, glider,
  Gosper glider gun, and deterministic random configurations.
- Escape closes the window. The complete legend is also rendered in-window.

## Implementation and evidence

The Life rule, toroidal neighbor shifts, presets, state transitions, camera
reducer, parametric torus projection, ray/surface picking, and line arrays are
MLPL. Rust selects the requested applet, normalizes bounded native input,
validates generic scene/view commands, and renders lines with winit/wgpu. It
contains no Life or torus rule.

`tests/test_life_torus.mlpl` proves births across a seam, a glider translating
through both seams, closed lattice geometry, and picking. The Rust applet test
proves the independently assembled MLPL source drives the generic host,
retains geometry for camera-only updates, acknowledges frames, and tears down.

The lattice contains 1,600 lines (two per cell); each live cell adds four
lines. Camera changes use retained `set_view` diffs and frame delivery is
single-flight. Cell changes and generations still replace the complete scene
until the queued generic ID-addressed scene-patch step lands.

The source and dependencies are conditional only through winit/wgpu's platform
backends and build targets; the same MLPL app and Rust host build on macOS and
Linux. This step has live macOS evidence and headless portable tests, but does
not claim a Linux visual run. A sphere remains future work because its poles
cannot honestly use the torus's rectangular two-axis wrap policy.
