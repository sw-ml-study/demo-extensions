# Native 3D weight-distribution explorer

Run `just weight-distribution`. The application starts with a bounded,
deterministically sorted menu of `.safetensors` and `.gguf` files beneath one
host-selected confined root. `MODEL_ROOT` may override the default
`../demo-ml-utils/models` root, but must be absolute. Symlinks are not followed.

Up/Down and Enter choose a model and then a tensor. Left or Backspace returns
to tensor selection; M returns to model selection. In the histogram view,
Up/Down selects a bin, left-drag orbits, Shift-left/middle-drag pans, the wheel
zooms, and R resets the camera. Opening a model first publishes a rotating
loading marker and names the file while the bounded catalog scan runs on the
MLPL worker. Stable IDs keep the eight histogram boxes, numeric count scale,
and color legend retained; bin selection sends only changed scene lines and
camera changes send view-only commands.

## What the view means

X spans the sampled minimum through maximum weight value. Y is
`0.35 + log2(sample count)` so one dense bin does not flatten smaller bins. The
status names the model, tensor, dtype, sampled/total values, bytes actually
read, minimum, mean, maximum, and zero count. World-space count labels mark
1, 4, 16, 64, 256, and 1,024 samples on the logarithmic Y axis. A matching
world-space legend labels blue `-` bins, orange `+` bins, and yellow `SEL`.
Blue covers the lower four value bins, orange the upper four, and yellow marks
the selected bin. The current
slice is a deterministic prefix sample, not a claim about the complete tensor
distribution; the UI says `SAMPLED`, never `ALL`, when the tensor exceeds 2,048
values.

MLPL owns file/tensor selection, decoder gating, histogram binning, statistics,
zero-count semantics, legends, camera intent, and renderer-neutral line IR.
Rust owns only confined model-path discovery and the existing generic window,
input, port, and retained-line renderer. Decoder/catalog source is reused at
build time from adjacent `demo-ml-utils`, so the demo exercises its public
Safetensors/GGUF contracts rather than inventing another format implementation.
For GGUF, the app retains published tensor-name offsets and lengths and performs
one bounded lazy read only when a name is displayed or selected.

## Bounds and evidence

- Discovery retains at most 64 relative model paths.
- Safetensors catalog reads are capped at 1 MiB and GGUF catalogs at 4 MiB,
  with 4,096 tensors/metadata entries, rank 8,
  and MLPL's exact-integer parameter ceiling.
- A selected Safetensors tensor reads at most 2,048 aligned integer values
  (4 KiB for I16/U16); GGUF reads at most 2,048 I8/I16 values or 64 complete
  Q8_0 blocks (2,176 bytes).
- Runtime state retains only one catalog, one capped decoded sample, eight bins,
  and a bounded stable line scene. Model-file size does not determine payload
  memory.
- Headless acceptance decodes eight real Safetensors I8 values and one real
  34-byte/32-value Q8_0 block from the shared deterministic fixtures. It also
  proves bounded sorted discovery and complete histogram conservation.

Run the focused evidence with:

```sh
./scripts/run-tests tests/test_weight_distribution.mlpl
cargo test -p mlpl-native3d-window --test weight_distribution_applet
```

The same winit/wgpu source runs on macOS and Linux. No conditional application
logic is required; winit/wgpu select their platform backends. Linux still needs
the normal graphics/window development libraries described by the repository.

Unsupported formats fail closed before payload reads. See
[`weight-distribution-blockers.md`](weight-distribution-blockers.md) for the
precise decoder ownership split and why the first slice requires no sw-MLPL
language change.

A real SmolLM2 Q8_0 acceptance run first exposed catalog correctness and memory
gates. `demo-ml-utils` commit `3310837` now reaches the exact tensor boundary
after 147,209 metadata-array elements in one second at 54,592 KiB peak RSS
under its 16 MiB standalone probe. This app adopts its lazy tensor-name-offset
contract. The larger composed interpreter applet still needs its explicit
bounded 64 MiB worker stack; that is distinct from catalog retained memory and
is recorded in the blocker matrix.
