# Native3D MLPL Library

Status: first reusable application layer, 2026-08-16

The files under `lib/native3d/` keep interactive application semantics in
ordinary MLPL while hiding the native window's command spelling and most
camera, picking, and bulk-line bookkeeping. They use only the public language
surface and can be included by cube, board-game, and grid demos.

## Modules

- `camera.mlpl` owns orbit camera state and reduces pointer drags and wheel
  input. Left drag orbits, middle drag or Shift-drag pans, and wheel input
  changes target distance. It also produces world-space pick rays and
  intersects them with application-selected planes.
- `geometry.mlpl` validates generic line batches, constructs parallel RGBA,
  thickness, and stable-ID arrays, and generates an XZ-plane grid.
- `retained.mlpl` compares generic stable-ID line scenes and emits bounded
  add/update/remove patch commands.
- `app.mlpl` includes both modules and supplies the generic transition and Port
  lifecycle. An application supplies initial state plus pure reducer and
  renderer callbacks. Older simple users may emit complete commands through
  this helper; interactive demos use `retained.mlpl` after initialization.

The `u:n3d_*` prefix is intentional: current ordinary user-defined MLPL
functions live in the `u` namespace. This is a source-level library, not a new
native namespace or a game-specific extension API.

## Data contracts

Line positions are dense `[N,3]` numeric arrays and edges are dense `[M,2]`
integer-valued indices. Colors are `[M,4]` RGBA values in `[0,1]`, thicknesses
and stable IDs are `[M]`. The initial implementation constructs owned MLPL
arrays and sends owned records across the Port; it makes no zero-copy claim.

Pointer coordinates and viewport dimensions are physical pixels with the
origin at the upper-left. Pick rays use the camera's vertical field of view
and viewport aspect ratio. Plane hits behind the camera and parallel rays are
reported as `hit:0`.

Camera and geometry helpers reject invalid dimensions, shapes, bounds, and
non-finite scalar inputs before constructing a native command. The renderer
still performs its own boundary validation, so malformed input fails closed on
both sides of the extension boundary.

## Application split

A concise demo should contain only its domain state, its event reducer, and a
function that maps state to generic line arrays. Rust remains responsible for
the portable winit/wgpu window, normalized input delivery, validated handles,
and rendering. The MLPL library owns camera gestures and native command
assembly but contains no cube, tic-tac-toe, or Life rules.

The native mlplunit suite `tests/test_native3d_library.mlpl` covers orbit,
zoom, pan, center picking, plane intersection, grid/line shapes, callback
transition, and scene-command assembly without opening a window.
