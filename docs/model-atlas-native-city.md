# Model Atlas native city

`just model-atlas` opens the first interactive native view of the versioned
Model Atlas interchange. It is a bounded proof of the visualization contract,
using the repository's derived seven-tensor Safetensors/GGUF fixture rather
than reading tensor payloads or an entire model file into memory.

## Controls

- Click a building to select it and show its tensor name, format, shape, dtype,
  parameter count, and stored byte count in the visible legend.
- Left-drag orbits and tilts. Shift-left-drag or middle-drag pans. The wheel
  zooms.
- `A`, `S`, and `G` show all tensors, Safetensors only, or GGUF only.
- `L` toggles between proportional building heights and low-detail footprints.
- `R` resets selection, filtering, level of detail, and camera. Escape closes.

## Data and visual encoding

The source fixture is
`fixtures/model-atlas/tensor_city_derived.mlpl`. Its provenance explicitly
identifies derived sample metadata; the app does not imply that it scanned a
user model. Safetensors and GGUF occupy separate labeled districts and use blue
and orange lines respectively. Selection changes a building to yellow.
Footprint area follows parameter count, height follows stored bytes through a
capped scale, and stable line IDs make selection, filtering, and LOD changes
atomic retained-scene patches instead of full geometry replacement.

The default fixture produces 92 lines: eight district-border lines and twelve
wireframe lines for each of seven tensor buildings. Filtering and LOD preserve
deterministic ordering and hard caps inherited from the validated interchange.
Later steps add architecture inference and bounded, on-demand tensor detail;
this step intentionally does neither.

## Ownership and portability

MLPL owns tensor interpretation, layout, colors, selection, filtering, LOD,
camera transitions, picking, stable IDs, and patch construction. Rust supplies
the generic winit event loop, wgpu line renderer, event transport, and retained
ID-addressed scene storage. There is no model-format or model-architecture
logic in Rust.

The same source builds on macOS and Linux through winit and wgpu. Platform
selection is handled by those crates rather than conditional application code.
Headless mlplunit and Rust integration tests are the acceptance evidence;
opening the desktop window is an opt-in smoke check and requires a usable local
graphics/display session.

## Current limits

This city uses a small checked-in derived fixture. Arbitrary model discovery
must first produce the bounded interchange using range reads and capped
summaries described in [bounded scanning](model-atlas-bounded-scan.md). The
renderer is wireframe-only, selected tensor text is displayed in the help
overlay rather than attached in world space, and a native retained patch is
still materialized as one contiguous GPU line buffer after validation. None of
those limitations require loading tensor payloads.
