# Native3D Interaction Contract

Status: live host slice, 2026-08-16

This contract is renderer-neutral and application-neutral. Rust normalizes
platform events and validates camera math; MLPL decides what a drag, wheel,
click, or frame means for a particular application.

## Coordinate and event records

Pointer coordinates are finite physical pixels with `[0,0]` at the viewport's
top-left, positive X right, and positive Y down. Events cross the Port as owned
records:

- `pointer_move`: `x`, `y`, current button flags, and modifiers;
- `pointer_down` / `pointer_up`: button, `x`, `y`, and modifiers;
- `wheel`: `dx`, `dy`, pointer `x`, `y`, and modifiers;
- `frame`: nonnegative `delta_ms` and `elapsed_ms`.

The live slice maps left/middle/right buttons and Shift, Control, Alt, and Meta
without assigning application behavior. winit pixel-wheel deltas are preserved;
line-wheel deltas use the documented 40-pixel normalization factor so MLPL sees
one platform-independent unit.

Discrete button transitions are never coalesced. Pending pointer moves replace
older pending moves, wheel deltas accumulate, and frames replace older pending
frames. A fixed-capacity queue fails explicitly when non-coalescible input
would overflow; the live host must apply the documented policy rather than
grow without bound.

## Orbit camera and picking

`OrbitCamera` contains a world target, yaw, pitch, positive target distance,
vertical field of view, and positive near plane. Pitch excludes the pole
singularity. All fields and derived rays must be finite.

`pick_ray(viewport,[x,y])` produces a normalized world-space ray. Screen center
points at the orbit target. Ray/plane intersection returns only forward hits;
parallel rays, invalid planes, and intersections behind the camera return no
hit. This lets MLPL map a generic plane hit to a tic-tac-toe square or Life
cell without a game-specific Rust picking call.

## Evidence and current limitation

`interaction_contract.rs` covers camera bounds, screen bounds, center-ray
orientation, plane hits, parallel rays, and behind-camera rejection.
`input_contract.rs` covers finite values, bounded overflow, coalescing order,
wheel accumulation, frame replacement, discrete-click preservation, and owned
MLPL record encoding.

The winit host queues cursor, button, and wheel events, flushes before capacity
overflow, and sends owned records through the existing Port. Frame delivery is
single-flight: after one frame is sent, redraws cannot send another until MLPL
returns `{op:"frame_ack",revision}` after consuming it. This bounds remote
frame backlog at one while discrete input remains ordered. Close/disconnect
still exits the event loop, and resize changes both the GPU surface and MLPL
viewport record.

Native logical keys are normalized into stable lowercase names before crossing
the Port. Tests pin every key advertised by the live Life legend; applet tests
that inject post-normalization records are not sufficient evidence for this
winit boundary.

Scene commands may include `camera:{target:[3],yaw,pitch,distance,fov,near}`.
The parser rejects malformed shapes, non-finite values, pole-singular pitch,
invalid clipping, and invalid field of view. Commands without `camera` retain
the backward-compatible default view. The same pure camera is consumed by CPU
headless planning and wgpu line planning.

The reusable ordinary MLPL layer now implements camera reduction, pick rays,
plane hits, generic grids, and the Port application lifecycle. Its contract and
test evidence are documented in [Native3D MLPL library](native3d-mlpl-library.md).
The cube, tic-tac-toe, and Life demos reuse this library without native
application-specific input maps.
