# Model Atlas native city

`just model-atlas` opens the first interactive native view of the versioned
Model Atlas interchange. It is a bounded proof of the visualization contract,
using the repository's derived seven-tensor Safetensors/GGUF fixture rather
than reading tensor payloads or an entire model file into memory.

## Controls

- Click a building to select it and show its tensor name and inferred role in
  the visible legend.
- Left-drag orbits and tilts. Shift-left-drag or middle-drag pans. The wheel
  zooms.
- `A` shows all tensors. `S` and `G` select Safetensors or GGUF respectively;
  pressing an already-active format key toggles directly back to All.
- `L` toggles between proportional building heights and low-detail footprints.
- `R` resets selection, filtering, level of detail, and camera. Escape closes.

## Data and visual encoding

The source fixture is
`fixtures/model-atlas/tensor_city_derived.mlpl`. Its provenance explicitly
identifies derived sample metadata; the app does not imply that it scanned a
user model. Safetensors and GGUF occupy separate labeled districts and use blue
and orange lines respectively. Selection changes a building to yellow.
Footprint area follows parameter count. Building height is
`0.5 + log2(stored_bytes + 1)` and the overlay supplies numeric ticks at 0, 1,
3, 7, and 31 bytes. This keeps orders-of-magnitude differences visible without
letting one tensor dominate the scene. Stable line IDs make selection,
filtering, and LOD changes atomic retained-scene patches instead of full
geometry replacement.

The default fixture produces 92 lines: eight district-border lines and twelve
wireframe lines for each of seven tensor buildings. Filtering and LOD preserve
deterministic ordering and hard caps inherited from the validated interchange.
The fixture's GGUF `general.architecture=mamba` value is displayed as
authoritative `[METADATA]`. Selected tensor roles are classified by explicit
MLPL name patterns and displayed separately as `[HEURISTIC]`; unknown or
ambiguous names remain `UNKNOWN`. Bounded, on-demand tensor detail is still a
later step. The complete classification contract is documented in
[architecture metadata and inference](model-atlas-architecture.md).

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
