# Headless native3d Provider

Date: 2026-08-15

The `_native3d` extension is the first real provider layer beneath the native
wireframe demo. It uses the same public ABI and safe SDK available to an
external crate and deliberately opens no window. Its job is generic resource,
array, scene, and render-state management; MLPL retains application semantics.

## Public primitives

- `create_viewer(width, height) -> native<Viewer>` creates a typed
  extension-scoped generational handle.
- `set_lines(viewer, positions, edges, colors, thicknesses, ids) -> record`
  replaces the complete generic line scene in bulk.
- `viewer_size(viewer) -> record` returns logical `width` and `height`.
- `viewer_state(viewer) -> record` returns vertex/line counts, frame,
  rotation, and whether a scene is configured.
- `render(viewer, rotation_y) -> record` records explicit application-provided
  rotation and advances a deterministic headless frame.
- `close(viewer) -> bool` invalidates and drops the resource.

Arrays are copied into extension-owned contiguous f64 storage at the boundary.
Positions have shape `[N,3]`; edges `[M,2]`; colors `[M,4]`; thicknesses and
IDs `[M]`. Counts are bounded. Values must be finite, indices must be integral
and in range, colors must be in `0..=1`, thicknesses positive, and IDs
nonnegative integers. This implementation makes no zero-copy claim.

## Acceptance evidence

`extensions/native3d/tests/provider.rs` proves parallel-array validation,
render-before-upload rejection, and host deactivation. The adjacent-host
`native3d_provider.rs` test registers the real descriptor through
`register_c_extension` and proves lifecycle, bulk upload, records, explicit
render state, close, stale generations, and malformed calls through the actual
sw-MLPL interpreter.

No native mlplunit suite can register this repository's private provider in the
installed interpreter yet; `just check` runs the real Rust host harness plus
the existing native mlplunit suites. Static provider startup is isolated to the
acceptance harness.

## Deliberate boundary

The provider contains no cube, key binding, scoring, or animation-loop
semantics. It is headless and deterministic on macOS and Linux. A later window
adapter can consume the same scene state, while the next saga step implements
the cube control reducer entirely in MLPL over synthetic structured events.
Real winit event delivery remains blocked on upstream event-loop ownership and
bounded polling support.
