# Native 3D weight-distribution explorer

Run `just weight-distribution`. The application starts with a bounded,
deterministically sorted menu of `.safetensors` and `.gguf` files beneath one
host-selected confined root. `MODEL_ROOT` may override the default
`../demo-ml-utils/models` root, but must be absolute. Symlinks are not followed.

Up/Down and Enter choose a model and then a tensor. Left or Backspace returns
to tensor selection; M returns to model selection. In the histogram view,
Up/Down selects a bin, left-drag orbits, Shift-left/middle-drag pans, the wheel
zooms, and R resets the camera. Stable IDs keep the eight histogram boxes and
24 axis/legend lines retained; bin selection sends only changed scene lines and
camera changes send view-only commands.

## What the view means

X spans the sampled minimum through maximum weight value. Y is
`0.35 + log2(sample count)` so one dense bin does not flatten smaller bins. The
status names the model, tensor, dtype, sampled/total values, bytes actually
read, minimum, mean, maximum, and zero count. Blue covers the lower four value
bins, orange the upper four, and yellow marks the selected bin. The current
slice is a deterministic prefix sample, not a claim about the complete tensor
distribution; the UI says `SAMPLED`, never `ALL`, when the tensor exceeds 2,048
values.

MLPL owns file/tensor selection, decoder gating, histogram binning, statistics,
zero-count semantics, legends, camera intent, and renderer-neutral line IR.
Rust owns only confined model-path discovery and the existing generic window,
input, port, and retained-line renderer. Decoder/catalog source is reused at
build time from adjacent `demo-ml-utils`, so the demo exercises its public
Safetensors/GGUF contracts rather than inventing another format implementation.

## Bounds and evidence

- Discovery retains at most 64 relative model paths.
- Catalog reads are capped at 1 MiB, 4,096 tensors/metadata entries, rank 8,
  and MLPL's exact-integer parameter ceiling.
- A selected Safetensors tensor reads at most 2,048 aligned integer values
  (4 KiB for I16/U16); GGUF reads at most 2,048 I8/I16 values or 64 complete
  Q8_0 blocks (2,176 bytes).
- Runtime state retains only one catalog, one capped decoded sample, eight bins,
  and 120 stable scene lines. Model-file size does not determine payload memory.
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

A real SmolLM2 Q8_0 acceptance run exposed a separate catalog gate: ordinary
GGUF tokenizer metadata arrays are not yet supported by `demo-ml-utils`, even
though Q8_0 payload blocks are. The app renders a red 3D X with the catalog
reason instead of leaving the prior flat placeholder or guessing payload
offsets. This limitation and the exact upstream requirement are recorded in
the blocker matrix.
