# MLPL Wireframe-Cube Controls

Date: 2026-08-15

The wireframe cube's application state and input behavior live entirely in
`demos/wireframe-cube/controls.mlpl`. Rust supplies generic handles, bulk line
storage, rendering, and eventually normalized native events; it does not know
which keys resize a cube or alter its style.

## State and controls

The reducer owns width, height, length, signed rotation speed, pause, palette
index and RGBA color, line thickness, drawable size, close intent, and a
monotonic revision. Normalized key names are:

| Key | MLPL behavior |
|---|---|
| W, S | Adjust width by 0.25 within `0.25..=100` |
| Up, Down | Adjust height by 0.25 within `0.25..=100` |
| D, A | Adjust length by 0.25 within `0.25..=100` |
| +, - | Adjust signed radians/second by 0.1 within `-10..=10` |
| Space | Toggle paused state without discarding signed speed |
| R | Restore application defaults while retaining drawable size |
| C | Advance the deterministic four-color MLPL palette |
| ], [ | Adjust pixels within `0.5..=20` |
| Escape | Request close |

Resize events have `{kind:"resize",width:number,height:number}` and clamp the
drawable dimensions to `1..=16384`. Close events have `{kind:"close"}`. Key
events have `{kind:"key",key:string}`. Other event kinds are deterministic
no-ops. This is the normalized record contract expected from future bounded
`poll_events`; platform-specific key codes must be normalized by the generic
window service before crossing into MLPL.

The winit adapter accepts Space as `NamedKey::Space` (and retains the character
fallback) and normalizes both forms to `"space"`. MLPL then toggles pause; a
second Space resumes using the unchanged signed speed.

## Bulk update

`u:wireframe_cube_bulk_update(state)` returns actual dense arrays ready for the
headless provider:

- positions `[8,3]`;
- edges `[12,2]`;
- colors `[12,4]`;
- thicknesses `[12]`;
- stable IDs `[12]`;
- effective signed rotation speed (zero while paused).

The same state yields an identical complete update. Native mlplunit covers all
controls, bounds, unknown events, resizing, close intent, reset, reverse
rotation, array shapes, stable IDs, pause/resume behavior, and determinism.

## Live event-loop connection

`just cube-3d` now opens winit/wgpu on the required main thread while sw-MLPL
runs this reducer on a worker. The UI sends one owned normalized event at a
time, and MLPL returns a complete owned scene command. The command also carries
the help text rendered at the top of the native view; the title displays the
MLPL revision and live dimensions/speed for explicit feedback.

The headless `live_applet.rs` host proves this same path without a display,
including physical W/S events. Compiled-provider startup and dynamic loading
remain separate future work.
