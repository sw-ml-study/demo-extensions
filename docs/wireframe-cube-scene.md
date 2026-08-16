# Wireframe Cube Scene Contract

The first native-3D slice is a rotating wireframe cube. It is deliberately a
small renderer fixture: MLPL owns the cube dimensions and computes all eight
vertices with bulk array arithmetic, while Rust accepts only a generic line
scene. No cube-specific rule belongs in the renderer.

## MLPL-owned inputs

`demos/wireframe-cube/scene.mlpl` exposes defaults and a validated scene
builder. Width, height, and length are independently adjustable in `(0, 100]`.
Rotation speed is signed radians per second in `[-10, 10]`; a negative value
reverses direction. Line color is four linear RGBA channels in `[0, 1]`. Line
thickness is in logical pixels in `[0.5, 20]`.

The defaults are a `2 x 2 x 2` cube, `0.6` radians per second, RGBA
`[0.2, 0.8, 1, 1]`, and a two-pixel line. The flat sign array is multiplied by
a parallel dimension array to produce row-major `[8,3]` positions. Twelve
index pairs form the `[12,2]` edge array. Native mlplunit tests prove scaling,
topology, defaults, validation, and repeatable serialization.

## Renderer-neutral interchange

The version-one JSON schema is `sw-ml-study.native3d.line-scene` and contains:

- `positions`: finite row-major numbers with shape `[N,3]`;
- `edges`: integer vertex indices with shape `[M,2]`;
- `controls`: rotation speed, RGBA line color, and logical-pixel thickness.

The Rust `mlpl-native3d-scene` crate rejects unsupported schemas and versions,
empty or inconsistent shapes, non-finite positions, out-of-range indices, and
unsafe controls. It limits scenes to one million vertices and two million
edges. Parsing owns a copy of the JSON data; no zero-copy claim is made.

Dense arrays now cross the real third-party provider boundary, and the MLPL
control reducer emits complete parallel arrays for `_native3d:set_lines`.
Deterministic JSON remains only the bridge used by the independently runnable
window smoke command until native event-loop delivery connects the two paths.
It is temporary transport, not an application semantic API.

## Interaction boundary

The native layer will report generic keyboard, resize, and close events. MLPL
already maps matching synthetic records to width, height, length, signed speed,
pause/reset, color, thickness, resize, and close state, then regenerates bulk
arrays. Rust contains no cube-specific key bindings or control policy. Only
live bounded event delivery remains before those tested pieces can form the
interpreted application loop; see
[`sw-mlpl-blockers.md`](sw-mlpl-blockers.md).

Headless parsing, geometry, projection, and draw-planning tests are required
acceptance evidence. Opening the wgpu/winit window is an opt-in smoke check and
will not be the only proof of correctness.
