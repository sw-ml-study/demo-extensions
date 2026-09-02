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
- grid changes emit bounded stable-ID `patch_scene` updates; only initialization
  or an explicit renderer resync emits a complete `set_scene` replacement.

The follow-up correction adds a generic `frame_ack` command. The host admits
no new frame while one is outstanding; MLPL acknowledges every consumed frame
after its transition and any resulting scene update. Discrete key and pointer
events can therefore have at most one frame ahead of them, so G/U cannot
starve behind stale animation frames. Clearing only a local queue would not
remove frames already delivered through the Port.

A second live report found that C and R worked while B/G/H/U appeared ignored.
This was not queueing: winit's native key normalizer used a deliberate
whitelist that omitted the Life-only letters, while the original worker test
injected `"g"` after normalization and therefore could not expose the bug. The
native test now pins B/G/H/I/N/T/U plus Space, and the whitelist forwards every
visible binding. MLPL also accepts winit's `equal`/`minus` spellings for the
displayed plus/minus speed controls.

The worker/Port regression proves that a run toggle produces no command, a
glider generation produces one stable-ID patch, and wheel input produces one
view update. The generic retained-scene protocol now sends four line upserts or
removals per changed cell and requests a complete MLPL snapshot after a rejected
patch. Rust still rebuilds its validated contiguous line scene after accepting
the diff; incremental GPU-buffer mutation remains future work.

## Memory and work bounds

The MLPL grid is capped at 256×256 and this demo uses 40×40. Input buffering is
fixed at 64 normalized events with coalescing for pointer and wheel traffic.
Frame delivery is single-flight across the Port, and each reducer frame
advances at most one generation. A scene
snapshot contains 82 grid lines plus four lines per live cell; ordinary cell
updates carry only changed-cell lines and view updates contain no geometry
arrays. All values crossing the Port are owned.

## Ownership

MLPL owns Life rules, topology, presets, timing, editing, gesture arbitration,
camera state, and geometry arrays. Rust owns normalized bounded events,
validated retained scene/view commands, native window/GPU resources, drawing,
and teardown. Neither `set_scene` nor `set_view` contains Life-specific logic.
