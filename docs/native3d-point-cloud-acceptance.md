# Native3D Point-Cloud Acceptance

Acceptance was run on 2026-09-02 on Darwin 25.5.0 arm64 with Rust 1.96.0.
The point-cloud slice passes its deterministic, bounded, native teaching goals.
This report separates observed behavior from source-level portability evidence.

## Acceptance matrix

| Criterion | Result | Evidence and limitation |
| --- | --- | --- |
| MLPL owns application semantics | Pass | Five native mlplunit cases cover generated arrays, exact-ID selection/no-hit behavior, camera reduction, monotonic revisions, patches, and close state. |
| Generic retained host | Pass | Rust tests cover complete/patch dispatch, atomic failures, budgets, view revisions, and authoritative overlap picking. No C ABI change was required. |
| Real worker flow | Pass | `point_cloud_applet` observes `set_points`, `patch_points`, `set_view`, frame acknowledgement, and clean channel teardown through the real evaluator. |
| Release build/package targets | Pass | `cargo build --release -p mlpl-native3d-window` passed. Cargo metadata exposes the library, native binary, `point_cloud_probe` example, and point acceptance tests from the workspace manifest. |
| macOS native startup | Pass with manual limit | `just point-cloud` built and launched the native process into its winit event loop on the host; it remained live until intentionally interrupted. This run is startup evidence, not automated visual/pixel or exhaustive control evidence. |
| Linux portability | Contract evidence only | The same cfg-free wgpu/winit source, shaders, bounded data contracts, and shell entry points target Linux, but no Linux runtime or cross-target toolchain was available in this session. |
| Offline deterministic input | Pass | The MLPL app generates a fixed 24-point helix and contains no filesystem, network, model, embedding, or PCA input path. |
| Repository boundary | Pass | The implementation and staged-path audit contain only files in `demo-extensions`; no sibling repository was modified. The existing public ABI/provider acceptance remains green. |
| Accessibility | Limited | Visible help documents controls and selection. There is no screen-reader bridge, keyboard point traversal, focus model, or nonvisual selection announcement in the native window. |

## Reproducible CPU evidence

Run `just point-cloud-acceptance`. The release-mode probe is capped at 100,000
points; the checked recipe uses 100, 1,000, and 10,000. It constructs owned
parallel arrays, runs camera projection/culling, and expands visible points to
the current six-vertex circular-sprite layout. A representative run on the host
reported:

| Requested/visible | Owned bytes | Upload-plan bytes | Screen-plan bytes | Expanded vertex bytes | Plan µs | Expand µs |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 / 100 | 4,400 | 4,000 | 4,000 | 24,000 | 5 | 4 |
| 1,000 / 1,000 | 44,000 | 40,000 | 40,000 | 240,000 | 36 | 20 |
| 10,000 / 10,000 | 440,000 | 400,000 | 400,000 | 2,400,000 | 412 | 350 |

Timings are elapsed CPU observations and vary by host/load. They are not a
statistical benchmark, frame-time guarantee, GPU upload measurement, GPU
throughput result, or Linux comparison. Byte counts are contractual for the
current owned (44 bytes/point), upload/screen plan (40 bytes/visible point), and
expanded vertex layout (240 bytes/visible point). The renderer owns fresh CPU
and GPU copies per frame; it makes no zero-copy claim.

## Commands run

```sh
./scripts/run-tests tests/test_point_cloud.mlpl
cargo test --release -p mlpl-native3d-scene --test point_headless_renderer \
  -p mlpl-native3d-window --test point_retained --test point_cloud_applet
cargo clippy -p mlpl-native3d-window --all-targets -- -D warnings
cargo build --release -p mlpl-native3d-window
just point-cloud-acceptance
just check
```

The interactive app remains `just point-cloud`; the lower-level JSON overlay is
`just point-cloud-smoke`. Neither interactive recipe belongs in unattended CI.

## Remaining scope

The point-cloud saga does not deliver real embeddings, PCA, model downloads,
GPU timing, instanced sprites, persistent GPU buffers, Linux runtime evidence,
or the accessibility features named above. Those are explicit later projects,
not implied by this acceptance result.
