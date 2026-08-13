# Wireframe Cube PoC Acceptance

Status date: 2026-08-13

| Capability | Status | Evidence | Limitation |
|---|---|---|---|
| MLPL-owned cube arrays and parameters | Proven | `tests/test_wireframe_cube_scene.mlpl` | JSON transport copies the scene |
| Generic scene validation | Proven | `scene_contract.rs` | No host array call yet |
| Transform, projection, clipping, and style | Proven | `headless_renderer.rs` | CPU rasterizer is test evidence, not the GPU backend |
| Deterministic headless image | Proven | fixed FNV-1a fingerprint over PPM bytes | Platform-independent by design |
| GPU thick-line expansion | Proven | `line_vertices.rs` | Basic PoC lines; no antialiasing promise |
| Native macOS window | Proven | observed `just cube-3d` run on 2026-08-13 | Manual opt-in smoke evidence |
| Native Linux design/build path | Supported, unverified here | shared winit/wgpu source and WGSL | No Linux target or graphical host was available on this Mac |
| MLPL-driven live controls | Blocked upstream | `docs/sw-mlpl-blockers.md` | Needs handles, events, arrays, and event-loop contract |
| Compiled MLPL application | Blocked upstream | `docs/sw-mlpl-blockers.md` | Needs compiler provider parity |

The PoC acceptance claim is intentionally narrow: a third-party-oriented Rust
renderer can consume an MLPL-generated generic scene and display a rotating
wireframe cube in a native desktop window. It does not claim the temporary file
bridge is the final extension API, that Linux was visually tested, or that
interactive cube policy currently executes in MLPL.
