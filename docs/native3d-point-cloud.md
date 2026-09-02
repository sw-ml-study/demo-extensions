# Native3D Point-Cloud Contract

Status: renderer-neutral contract, deterministic headless renderer, native
wgpu/winit point pipeline, generic retained updates/selection events, and the
deterministic MLPL teaching application are delivered.

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
alpha blending. Stable IDs are copied into the GPU buffer for identity
continuity, while selection uses the matching bounded CPU plan and requires no
GPU readback.

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
pan, zoom, and rotation continue through the existing camera path; a left-button
release emits the generic selection event, though the wireframe smoke applet does
not assign it application meaning.

## Retained updates and selection delivery

MLPL can replace point state with `set_points` and atomically update it with
`patch_points`. Complete commands carry parallel `positions`, `sizes`, `colors`,
`opacities`, and `ids` arrays plus a revision, camera, help, and optional status.
Patches carry `base_revision`, an advancing `target_revision`, parallel upsert
arrays, and `remove_ids`. A patch may describe at most 100,000 operations and
the resulting scene may retain at most 100,000 points. Duplicate/conflicting
IDs, stale revisions, unknown removals, invalid attributes, empty results, and
budget overruns leave prior state unchanged; rejected live patches request a
complete resynchronization. Geometry-preserving `set_view` commands advance the
same retained revision, so subsequent selection events identify the visible
view consistently.

On left-button release, the window projects the current retained point scene
through the current physical-pixel viewport and sends an owned
`point_selection` record. `hit` is numeric zero/one, while `id` and `revision`
are decimal strings; no-hit uses an empty ID string. Strings preserve the full
generic `u64` identity without rounding through MLPL's `f64` arrays. Incoming
command IDs remain non-negative integral MLPL array values and are rejected
above the exactly representable integer range. The event reports identity
only: MLPL decides whether it means selection, inspection, filtering, or no
application action.

## Evidence and remaining scope

`point_scene_contract.rs` starts red against the absent API and covers valid
planning, exact attribute/order preservation, malformed and non-finite data,
parallel length mismatches, duplicate identity, and independent count/byte
limits. `point_headless_renderer.rs` pins near/offscreen culling, far-to-near
ordering, stable-ID overlap/picking ties, rotation validation, attribute-sensitive
pixels, and a deterministic PPM hash. Existing line-scene tests remain unchanged.

`point_retained.rs` covers complete/patch command dispatch, atomic add/update/
remove behavior, stale and unknown IDs, conflict and final-scene budget failures,
and exact-ID hit/no-hit event encoding under the authoritative overlap policy.

The deterministic MLPL teaching application is documented in
`native3d-point-cloud-demo.md`. The embedding/PCA application remains
unimplemented. GPU picking/readback is deliberately unnecessary in this slice:
the same bounded CPU render plan determines both visible ordering and selection.
