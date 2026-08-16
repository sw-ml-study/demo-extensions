# Native3D Interaction Contract

Status: first headless contract slice, 2026-08-16

This contract is renderer-neutral and application-neutral. Rust normalizes
platform events and validates camera math; MLPL decides what a drag, wheel,
click, or frame means for a particular application.

## Coordinate and event records

Pointer coordinates are finite logical pixels with `[0,0]` at the viewport's
top-left, positive X right, and positive Y down. Events cross the Port as owned
records:

- `pointer_move`: `x`, `y`, current button flags, and modifiers;
- `pointer_down` / `pointer_up`: button, `x`, `y`, and modifiers;
- `wheel`: `dx`, `dy`, pointer `x`, `y`, and modifiers;
- `frame`: nonnegative `delta_ms` and `elapsed_ms`.

The initial slice models left/middle/right buttons and Shift without assigning
application behavior. Step 2 will map the full winit state into this contract.

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

This step does not yet connect winit events or change the GPU camera. Those are
the next saga step. The existing keyboard cube remains functional meanwhile.
