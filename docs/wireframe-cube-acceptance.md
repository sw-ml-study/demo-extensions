# Wireframe Cube PoC Acceptance

Status date: 2026-08-15

| Capability | Status | Evidence | Limitation |
|---|---|---|---|
| MLPL-owned cube arrays and parameters | Proven | `tests/test_wireframe_cube_scene.mlpl` | Live window still uses the JSON smoke bridge |
| Generic scene validation | Proven | `scene_contract.rs`, `native3d_provider.rs` | Host bulk call is headless until live event delivery |
| Transform, projection, clipping, and style | Proven | `headless_renderer.rs` | CPU rasterizer is test evidence, not the GPU backend |
| Deterministic headless image | Proven | fixed FNV-1a fingerprint over PPM bytes | Platform-independent by design |
| MLPL-owned interactive reducer | Proven with synthetic records | `test_wireframe_cube_controls.mlpl` | Live winit polling remains upstream-blocked |
| Deterministic provider bulk update | Proven | `[8,3]`, `[12,2]`, `[12,4]`, parallel thickness/ID arrays | Application semantics remain in MLPL |
| GPU thick-line expansion | Proven | `line_vertices.rs` | Basic PoC lines; no antialiasing promise |
| Native macOS window | Proven | observed `just cube-3d` run on 2026-08-13 | Manual opt-in smoke evidence |
| Native Linux design/build path | Supported, unverified here | shared winit/wgpu source and WGSL | No Linux target or graphical host was available on this Mac |
| MLPL-driven live controls | Reducer proven; delivery blocked upstream | `test_wireframe_cube_controls.mlpl` | Needs native event-loop ownership and bounded polling |
| Compiled MLPL application | Blocked upstream | `docs/sw-mlpl-blockers.md` | Needs compiler provider parity |

The PoC acceptance claim is intentionally narrow: a third-party-oriented Rust
renderer can consume an MLPL-generated generic scene and display a rotating
wireframe cube in a native desktop window; separately, the real provider and
MLPL control policy are executable headlessly. It does not claim the temporary
file bridge is the final extension API, that Linux was visually tested, or
that real native events reach the MLPL reducer yet.
