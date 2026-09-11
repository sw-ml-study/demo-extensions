# Native3D filled-box contract

The generic filled-box scene is the renderer primitive used by later system
layout demonstrations. Rust assigns no storage, memory, filesystem, SWTOS, or
state meaning to a box. MLPL supplies all positions, dimensions, colors, and
stable identities.

## Bulk arrays

Version 1 uses schema `sw-ml-study.native3d.box-scene` and four parallel arrays:

- row-major finite `centers` with shape `[N,3]`;
- row-major finite, strictly positive `sizes` with shape `[N,3]`;
- linear RGBA `colors` with shape `[N,4]`, represented in JSON as N rows, with
  every finite channel in `0..=1`; and
- unique unsigned 64-bit stable `ids` with shape `[N]`.

N must be nonzero. The caller supplies nonzero `BoxLimits` for count and owned
source bytes, with a hard maximum of 100,000 boxes. Source accounting is 48
bytes per box: six `f32` center/size values, four `f32` color values, and one
`u64` ID. JSON parsing and the array constructor copy and own every accepted
value. There is no borrowed lifetime, zero-copy claim, GPU allocation, or ABI
layout promise.

## Triangle and headless plans

Each box expands in input order to eight corners and twelve consistently wound
triangles. The backend-neutral plan owns 36 vertices per box and reports its
materialized byte count using a conceptual vertex of `[3] f32` position,
`[4] f32` color, and `u64` identity. This deliberately simple expansion is a
correctness oracle; a native backend may later use indexed or instanced draws
without changing the scene contract.

The pure camera path applies Y rotation and perspective projection, discards
triangles crossing the near plane, removes triangles wholly outside the
viewport, and sorts accepted triangles far-to-near by average view depth.
Exact depth ties draw larger stable IDs first, so lower IDs are deterministically
topmost. The CPU rasterizer uses triangle coverage and source-over alpha on the
shared bounded RGBA surface. It is deterministic evidence, not an antialiasing
or hardware-depth equivalence claim.

## Native window

The opt-in winit/wgpu path converts every projected triangle into three owned
52-byte `GpuBoxVertex` records. Each record retains normalized XYZ, RGBA,
barycentric edge coordinates, the low/high halves of the full `u64` stable ID,
and a selected flag. The box pipeline writes and compares `Depth32Float` depth;
the depth attachment is recreated whenever the surface is resized. Boxes draw
before existing point, line, help, and status overlays.

A selected ID leaves its semantic RGBA unchanged and draws a thin gold edge in
the fragment shader. This provides deterministic selection visibility for an
ID supplied by the caller; interactive ray picking is the next contract step.
The current simple transparent path writes depth, so it is intended for opaque
or mostly opaque teaching layouts rather than order-independent transparency.

On a graphical macOS or Linux session, run:

```sh
just box-scene-smoke
```

The command opens the synthetic neutral box fixture and outlines ID 17. It does
not yet consume the columnar SWTOS artifact: `sw-mlpl` owns the array mapping,
and the later dynamic-API/integration steps will connect its `set_boxes`
output without placing storage meanings in Rust.

The synthetic fixture is
[`fixtures/native3d-box-scene.json`](../fixtures/native3d-box-scene.json).
Focused tests cover owned planning, exact triangle counts and IDs, shapes,
finite values, positive sizes, RGBA bounds, parallel lengths, duplicate IDs,
independent count/byte limits, stable depth order, occlusion, and repeatable
headless pixels.
