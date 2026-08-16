# Native Life Plane Acceptance

Status date: 2026-08-16

| Capability | Status | Evidence | Limitation |
|---|---|---|---|
| Finite B3/S23 evolution | Proven | `test_life_model.mlpl` | Dead edges, not toroidal |
| Presets and seeded random | Proven | model fixtures | Centered replacement semantics |
| Click and drag editing | Proven headlessly and live | controls suite and macOS use | Ctrl-left paints; plain drag controls camera |
| Run, pause, step, clear, speed | Proven | controls and worker/Port suites | At most one generation per frame |
| Orbit/tilt, pan, zoom | Proven headlessly and live | shared camera suite and macOS use | Physical-pixel gesture threshold |
| Generic bulk scene | Proven | scene mlplunit and Rust parser | Wire outlines rather than filled cells |
| Retained view updates | Proven | `life_applet.rs` | Camera/help diff only in this slice |
| Bounded delivery and teardown | Proven | input contract and applet close | Local interpreted applet |
| macOS native window | Proven | `just life-3d` | Manual visual evidence |
| Linux | Design/build supported | winit/wgpu portable source | Not visually run on this Mac |
| Compiled MLPL binary | Blocked upstream | `sw-mlpl-blockers.md` | Compiler/provider startup parity |

## Responsiveness correction

The first live build rebuilt and retransmitted all 1,600 grid cells for every
frame and pointer move, including while paused. Interactive use exposed severe
queueing and input lag. The corrected protocol retains the current geometry in
the native host:

- unchanged reducers emit no command;
- idle pointer motion is an MLPL no-op;
- paused frames and run/speed-only state changes emit no geometry;
- camera motion emits a validated `set_view` command containing only camera,
  revision, and help; and
- grid changes emit a complete `set_scene` replacement.

The worker/Port regression proves that a run toggle produces no command, a
glider generation produces one scene replacement, and wheel input produces one
view update. This is the first retained-scene, or “shadow DOM,” slice. A future
generic ID-addressed patch protocol should update changed line/cell objects
without rebuilding a full scene. Until that exists, generation and click
updates still pay the complete MLPL geometry cost; camera and idle motion do
not.

## Memory and work bounds

The MLPL grid is capped at 256×256 and this demo uses 40×40. Input buffering is
fixed at 64 normalized events with coalescing for pointer, wheel, and frame
traffic. Each reducer frame advances at most one generation. A scene
replacement contains 82 grid lines plus four lines per live cell; view updates
contain no arrays. All values crossing the Port are owned.

## Ownership

MLPL owns Life rules, topology, presets, timing, editing, gesture arbitration,
camera state, and geometry arrays. Rust owns normalized bounded events,
validated retained scene/view commands, native window/GPU resources, drawing,
and teardown. Neither `set_scene` nor `set_view` contains Life-specific logic.
