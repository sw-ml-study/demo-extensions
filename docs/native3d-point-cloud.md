# Native3D Point-Cloud Contract

Status: renderer-neutral contract delivered; GPU/window and application slices
remain later AgentRail steps.

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

## Evidence and remaining scope

`point_scene_contract.rs` starts red against the absent API and covers valid
planning, exact attribute/order preservation, malformed and non-finite data,
parallel length mismatches, duplicate identity, and independent count/byte
limits. Existing line-scene, camera, clipping, and headless raster tests remain
unchanged.

The next slice may project/rasterize these generic points headlessly. Native
wgpu/winit integration, typed viewer handles, retained point patches, picking,
MLPL fixtures, and the embedding/PCA application are not implemented by this
contract step.
