# Wireframe Cube PoC Acceptance

Status date: 2026-08-15

| Capability | Status | Evidence | Limitation |
|---|---|---|---|
| MLPL-owned cube arrays and parameters | Proven live | mlplunit plus `live_applet.rs` | Owned channel copies; no zero-copy claim |
| Generic scene validation | Proven | `scene_contract.rs`, `native3d_provider.rs`, `live_applet.rs` | Current GPU slice renders uniform per-scene style |
| Transform, projection, clipping, and style | Proven | `headless_renderer.rs` | CPU rasterizer is test evidence, not the GPU backend |
| Deterministic headless image | Proven | fixed FNV-1a fingerprint over PPM bytes | Platform-independent by design |
| MLPL-owned interactive reducer | Proven live | `test_wireframe_cube_controls.mlpl`, interactive window | Rust only normalizes physical keys |
| Deterministic provider bulk update | Proven | `[8,3]`, `[12,2]`, `[12,4]`, parallel thickness/ID arrays | Application semantics remain in MLPL |
| GPU thick-line expansion | Proven | `line_vertices.rs` | Basic PoC lines; no antialiasing promise |
| Native macOS window | Proven | observed interactive `just cube-3d` run on 2026-08-15 | Manual opt-in smoke evidence |
| Native Linux design/build path | Supported, unverified here | shared winit/wgpu source and WGSL | No Linux target or graphical host was available on this Mac |
| MLPL-driven live controls | Proven | `live_applet.rs` plus manual window | Local interpreted mode |
| In-view help and live feedback | Proven | bitmap overlay plus revision/state title | Compact PoC typography |
| Compiled MLPL application | Blocked upstream | `docs/sw-mlpl-blockers.md` | Needs compiler provider parity |

The PoC acceptance claim is intentionally narrow: a third-party-oriented Rust
renderer can consume MLPL-generated bulk scenes and display a live interactive
wireframe cube while the MLPL worker owns application behavior. It does not
claim Linux was visually tested, dynamic loading, compiled parity, true unload,
or zero-copy transport.
