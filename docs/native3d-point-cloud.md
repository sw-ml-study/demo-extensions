# Native3D Point-Cloud Contract

Status: renderer-neutral contract, deterministic headless renderer, and native
wgpu/winit point pipeline delivered; retained updates, picking events, and the
MLPL application slice remain later AgentRail steps.

## V1 scene

The version-one schema is `sw-ml-study.native3d.point-scene`. It contains
finite row-major `positions` with shape `[N,3]` plus parallel `[N]` sizes,
`[N,4]` linear RGBA colors, `[N]` opacities, and `[N]` stable non-negative
integer IDs. Point order is retained exactly; IDs provide application-neutral
identity for later selection and retained updates.

Sizes are logical pixels in `0.5..=256`. Every color channel and opacity is
finite and in `0..=1`. The upload plan multiplies color alpha by the parallel
opacity once. Keeping opacity separate at the scene boundary lets MLPL map a
scalar array independently without giving Rust cluster, score, embedding, or
selection semantics.

## Bounds and ownership

The caller must supply nonzero `PointLimits` for both point count and owned
source bytes. The implementation hard cap is one million points. Budgeting is
checked before upload planning with 44 bytes per point: three `f32` positions,
one `f32` size, four `f32` color channels, one `f32` opacity, and one `u64` ID.
Duplicate IDs, mismatched lengths, malformed shapes, non-finite values, and
budget overruns fail closed.

Parsing owns all input arrays. The deterministic backend-neutral upload plan
owns a second ordered point vector and reports 40 bytes per record after
opacity is folded into alpha. This is accounting evidence, not a Rust ABI,
wgpu vertex layout, zero-copy promise, or GPU allocation. Those decisions are
intentionally deferred to the renderer step.

## Headless projection, ordering, and picking

The pure point planner uses the shared validated orbit-camera convention and
physical-pixel viewport. It applies optional Y rotation, rejects points behind
the near plane, and culls circles wholly outside the viewport before producing
an output vector. Retained IDs survive projection.

Transparent points are ordered far-to-near. Exact depth ties are painted with
larger IDs first, making the lowest stable ID topmost; picking walks the same
circle coverage in reverse draw order. This deterministic tie policy avoids
depending on input order and adds no domain meaning to IDs.

The CPU evidence renderer draws bounded circular sprites with source-over alpha
onto the same fixed background used by line evidence. It is a regression oracle,
not the future wgpu implementation or a promise about antialiasing.

## Native GPU layout and smoke check

`point_vertices` expands each visible, depth-ordered point to six triangle
vertices. `GpuPointVertex` is a 40-byte `repr(C)`/bytemuck record: two `f32`
normalized-device coordinates, four `f32` color channels, two `f32` local
circle coordinates, and the low/high `u32` halves of the stable ID. The fragment
shader discards square corners outside the unit circle and uses source-over
alpha blending. Stable IDs are copied into the GPU buffer for the later picking
slice; this step does not expose a GPU readback or picking event.

The backend creates a fresh owned vertex copy for each frame, after bounded CPU
projection and culling. That is 240 uploaded bytes per visible point in this
small teaching implementation. It is intentionally not described as zero-copy
or an optimized instanced layout. Empty plans create no point buffer. The window
path rejects smoke JSON above 64 MiB before parsing, uses wgpu/winit on macOS and
Linux, and leaves the existing line, camera, lifecycle, input, and ABI boundaries
intact.

On a graphical macOS or Linux session, run:

```sh
just point-cloud-smoke
```

This opens the existing MLPL-owned wireframe applet with the bounded generic
fixture in `fixtures/native3d-point-scene.json` rendered through the point
pipeline. The check is intentionally opt-in because CI may be headless. Orbit,
pan, zoom, and rotation continue through the existing camera path; point
selection is not wired yet.

## Evidence and remaining scope

`point_scene_contract.rs` starts red against the absent API and covers valid
planning, exact attribute/order preservation, malformed and non-finite data,
parallel length mismatches, duplicate identity, and independent count/byte
limits. `point_headless_renderer.rs` pins near/offscreen culling, far-to-near
ordering, stable-ID overlap/picking ties, rotation validation, attribute-sensitive
pixels, and a deterministic PPM hash. Existing line-scene tests remain unchanged.

Retained point patches, pointer-to-ID event delivery, MLPL point application
semantics, and the embedding/PCA application remain unimplemented.
