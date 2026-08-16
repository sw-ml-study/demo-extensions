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
| `width_up`, `width_down` | Adjust width by 0.25 within `0.25..=100` |
| `height_up`, `height_down` | Adjust height by 0.25 within `0.25..=100` |
| `length_up`, `length_down` | Adjust length by 0.25 within `0.25..=100` |
| `speed_up`, `speed_down` | Adjust signed radians/second by 0.1 within `-10..=10` |
| `pause` | Toggle paused state without discarding signed speed |
| `reset` | Restore application defaults while retaining drawable size |
| `color_cycle` | Advance the deterministic four-color MLPL palette |
| `thickness_up`, `thickness_down` | Adjust pixels within `0.5..=20` |
| `close` | Set sticky close intent |

Resize events have `{kind:"resize",width:number,height:number}` and clamp the
drawable dimensions to `1..=16384`. Close events have `{kind:"close"}`. Key
events have `{kind:"key",key:string}`. Other event kinds are deterministic
no-ops. This is the normalized record contract expected from future bounded
`poll_events`; platform-specific key codes must be normalized by the generic
window service before crossing into MLPL.

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
rotation, array shapes, stable IDs, pause behavior, and determinism.

## Final event-loop seam

Only live delivery remains. The host/window integration must open the native
viewer on the platform-required thread and expose bounded ordered polling that
returns the records above. The MLPL application then folds each event through
`u:wireframe_cube_reduce`, calls `u:wireframe_cube_bulk_update` when its
revision changes, passes arrays to `_native3d:set_lines`, supplies explicit
rotation to `_native3d:render`, and closes when requested.

Synthetic records make this logic executable and deterministic today. They do
not claim real winit delivery, callback reentrancy, queue backpressure, or
compiled-provider startup.
