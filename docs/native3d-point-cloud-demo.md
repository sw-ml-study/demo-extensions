# Native3D Point-Cloud Demo

Run `just point-cloud` on a graphical macOS or Linux session. The native window
shows a deterministic 24-point synthetic helix; it performs no network access,
model download, embedding calculation, PCA, or filesystem read.

## Interaction

- Left click selects the topmost stable point ID and highlights it in gold.
- Left drag orbits; Shift+left drag or middle drag pans; the wheel zooms.
- `R` resets the camera and selection. `Esc` closes the app.

The help overlay names the selected ID and reminds the viewer that this is a
generic bulk-array teaching example rather than an embedding visualization.
The graphical run is opt-in because automated environments may be headless.

## Responsibility boundary

MLPL generates the `[24,3]` positions, `[24]` sizes, `[24,4]` colors, `[24]`
opacities, and `[24]` stable IDs. It owns selection meaning, gold highlighting,
camera reduction, revision monotonicity, reset/close behavior, complete
`set_points` commands, atomic `patch_points` styling updates, and `set_view`
commands. Selection currently resubmits all 24 point attributes in one bounded
patch for clarity; it is deterministic, not a claim of minimal bandwidth.

Rust provides application-neutral command validation, retained transaction
limits, physical-pixel projection and hit testing, winit input normalization,
wgpu circular sprites, and owned event delivery. A left release produces a
generic exact-string `point_selection` event; MLPL decides how to interpret it.
No C ABI or native extension function contains point-cloud, helix, selection,
embedding, or PCA semantics.

## Evidence and limits

`tests/test_point_cloud.mlpl` covers deterministic bounded arrays, exact-string
selection/no-hit transitions, camera behavior, monotonic revisions, patches,
and teardown through native mlplunit. The Rust `point_cloud_applet` integration
test runs the real MLPL worker headlessly and proves complete scene, selection
patch, view, frame acknowledgement, and close behavior through generic host
commands.

The demo is capped at 24 generated points while the generic retained host cap is
100,000. The renderer still expands each visible point to six vertices and
uploads a fresh owned buffer per frame. Accessibility beyond the visible help
overlay and a real embedding/PCA application remain future work. Measured CPU
evidence, package checks, platform scope, and precise limitations are recorded
in `native3d-point-cloud-acceptance.md`.
