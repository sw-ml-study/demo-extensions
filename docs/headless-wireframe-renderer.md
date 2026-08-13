# Headless Wireframe Renderer

`mlpl-native3d-scene` contains a renderer-independent pipeline shared by
headless tests and the future wgpu backend. It consumes only the generic
`[N,3]` position and `[M,2]` edge arrays; it has no cube topology, key mapping,
or application policy.

This is intentionally a first-pass proof of concept: it demonstrates how a
third-party extension can provide the primitives for an interactive native-3D
MLPL application. More sophisticated cameras, antialiasing, rendering quality,
input schemes, and performance work are follow-on refinements, not hidden
requirements of the initial cube.

The pipeline rotates positions around the vertical axis, moves them into camera
space, clips edges against the near plane, applies a vertical-field-of-view
perspective projection, and clips projected lines to the viewport. Output is a
list of pixel-space line endpoints with the scene's RGBA color and logical-pixel
thickness. A wide viewport preserves pixel scale while shifting the optical
center, avoiding aspect-ratio stretching.

The deterministic CPU rasterizer exists for tests and portable evidence. It
draws thick antialiased-independent line coverage into a bounded RGBA8 buffer
and can encode binary PPM without image-library or GPU dependencies. It is not
the interactive renderer and makes no performance claim. Tests cover rotation,
aspect ratio, dimensions, color, thickness, near and viewport clipping,
degenerate edges, allocation bounds, invalid cameras, non-finite rotation, and
byte-for-byte reproducibility.

## Interactive ownership

The native extension API will expose generic primitives: create a window,
poll bounded input/resize/close events, update bulk line arrays and style,
render/present, query monotonic time, and close a typed viewer handle. MLPL—not
Rust—will map those events to cube dimensions, rotation speed, line color, or
thickness and will own the application loop. Rust owns only platform event
collection, resource safety, and rendering.

That live split requires upstream persistent handles, event polling, bulk array
updates, host event-loop integration, and compiled extension parity. Until
those contracts ship, the headless pipeline and later visual smoke executable
can be developed and tested, but a Rust-owned cube control loop is not accepted
as the final interactive MLPL demo.
