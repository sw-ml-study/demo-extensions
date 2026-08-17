# Model Atlas: real local files

`just model-atlas-file` opens a native model picker before rendering any
tensor geometry. By default it searches the adjacent
`demo-ml-utils/models` directory for Safetensors files. Set `MODEL_ROOT` to
an absolute directory to inspect a different confined tree.

Use Up/Down to choose a file; the selected path is repeated in yellow below
the menu. Enter first displays an explicit bounded-analysis state and then the
atlas. Left-drag orbits/tilts, Shift-left-drag or middle-drag pans, the wheel
zooms, R resets the camera, and M returns to the picker. Escape closes the
window. The first slice supports Safetensors only; GGUF selection will follow
after its bounded catalog is integrated.

Menu rows show file size and an MLPL-formatted last-modified UTC date through
the confined `file_metadata` primitive shipped in sw-MLPL commit `0f4d0e32`.
Fixed-epoch tests cover 1970, a non-current 2020 date, and leap day. Compiled
parity and the follow-on `demo-file-processing` adoption are documented in
[sw-MLPL blockers](sw-mlpl-blockers.md#confined-filesystem-modification-times--open).

The MLPL application owns discovery, selection, bounded catalog parsing,
logarithmic byte-height mapping, scene construction, and navigation. The Rust
host supplies the native window, generic line renderer, event transport, and
one canonical filesystem root. It does not contain model-format or tensor
semantics.

Analysis reads the eight-byte Safetensors length prefix and at most a 1 MiB
JSON header. JSON decoding is capped at 200,000 elements, the menu displays and
permits selection from at most 12 discovered paths, and the initial atlas renders at most 32 tensor
records. Tensor payloads are not read. Consequently the model's total payload
size does not determine this view's resident data.

The current `fs_walk` host primitive returns the complete matching path list;
the 12-entry bound applies after discovery, not to traversal itself. Point the
demo at a deliberately scoped model directory. A host-level maximum-result
option would be needed before claiming bounded discovery of an arbitrarily
large tree.

The catalog functions are reused at build time from the adjacent
`demo-ml-utils` checkout. This deliberately demonstrates composition across
the study repositories, but it is not yet a standalone package dependency.
The current downstream host adapter configures sw-MLPL's public `Environment`
with a canonical root because the parked-main helper has no configured-root
entry point; the upstream parity request and containment requirements are in
[sw-MLPL blockers](sw-mlpl-blockers.md).

Headless acceptance covers confined discovery, the initial menu, bounded
analysis of a real-format fixture, the resulting scene, return to the menu,
and the unchanged no-filesystem default. Interactive smoke testing uses the
same winit/wgpu path on macOS and Linux.
