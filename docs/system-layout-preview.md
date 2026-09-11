# SWTOS system-layout preview

`just system-layout-swtos` opens a native `wgpu`/`winit` window containing
the hash-pinned authoritative layout published by SWTOS. It exercises this
repository's MLPL consumer and generic renderer against real producer data.

The current SWTOS artifact contains one external W25Q32 `flash` space. Its
4,194,304-byte capacity is storage, not the system's RAM address space. The
viewer expands the 49 occupied or padding eight-byte blocks into small cubes
arranged in 4-by-4 layers. It does not generate 524,242 empty cubes: the free
region is one separate logarithmically sized summary. An `embedded-hello`
block starts selected. Click any cube or the free summary to select its region.

The viewer is stationary by default. Click the visible `Rotate` control or
press `R` to toggle automatic rotation. Click the `Kind` or `Owner`
controls (or press `1` or `2`) to recolor every box. Press `H` repeatedly to
isolate legend categories in the current mode, and `C` to restore all regions.
The colored legend at the right changes with the selected mode. The top overlay
documents every shortcut. Selection details appear separately in a 1.5-times
scale amber callout at bottom-left. Use Left/Right arrows or `[`/`]` to walk
through visible blocks with wraparound. Drag to orbit, Shift-drag to pan, use
the wheel to zoom, and press Escape to close the window.

## Run it

From this repository:

```sh
just system-layout-swtos
```

The older `just system-layout-preview` name remains as a compatibility alias.

For a repeatable recording, leave the viewer running and use a second terminal:

```sh
just system-layout-swtos-tour
```

Focus the viewer during the one-second countdown. On macOS, the first key is
`R`, so rotation is visible throughout `1`, `2`, `H`, `H`, `H`, Right, Right,
Right. A final `R` stops rotation on a stable frame. The complete countdown and
tour take about eight seconds, fitting inside a ten-second recording. The
terminal application must have Accessibility permission to control the focused
window. The tour never closes or mutates the source artifact.
The generic alias is `just system-layout-tour`; matching
`system-layout-mlos-tour` and `system-layout-mesaos-tour` recipes are already
reserved for the next two producer integrations.

There is no setup or generated data to retain. The launcher selects the
adjacent development MLPL binary (or the absolute `MLPL` override), evaluates
[`scene.mlpl`](../demos/system-layout-preview/scene.mlpl), writes its generic
box-scene JSON to a temporary directory, and launches the native3d window.

MLPL owns JSON parsing, kind/owner/space palettes, legend content, region
annotations, space placement, block geometry, stable IDs, and scene
construction. Rust owns bounded presentation validation, generic text/control
rendering and hit testing, bulk box rendering, camera input, and selection
highlighting. The native code has no knowledge of OS region kinds or owners.

## Provenance and limits

SWTOS emitter and validator revision `5c4a24e` produced the clean-tree artifact
published at SWTOS revision `a08d886`. This repository vendors that file
byte-for-byte as
[`fixtures/swtos-storage-layout-5c4a24e.json`](../fixtures/swtos-storage-layout-5c4a24e.json),
SHA-256 `58d29496c75efaa95567d0208ead48dc4c83797c86f2a6952dfc78d1ce748a84`.
The launcher refuses to run if it changes unnoticed. Producer evidence reports
8 regions, 3 `describes` edges, 10 programs, and 372 of 4,194,304 flash bytes
used; the downstream tests independently pin the region extents, IDs, and
relationship targets.

Real producer acceptance exposed a palette gap that the earlier synthetic
fixture did not: SWTOS preserves two alignment regions as `padding`, distinct
from `free`. The MLPL palette now covers the complete closed vocabulary and
renders padding separately. MLOS and MesaOS will use the same producer-neutral
columnar contract in later steps.

This artifact is intentionally a storage-only view. External flash, system
RAM, and EBR are distinct devices/address spaces and must render as separate
groups when present. A process stack is a region owned within RAM or EBR, not
a peer device. SWTOS runtime RAM/EBR/stack data and `loads-to` relationships
belong to the separately queued runtime-snapshot step; they are not inferred
or fabricated from this flash artifact.

The native preview now shows bounded block layers, a prominent selected-region
annotation, and switchable color legends. Multiple selectable cubes share one
region annotation through a generic presentation index; this avoids duplicating
application strings at the native boundary. Relationship edges and labels
anchored directly to each box remain follow-up presentation work; the fixture
already carries the relationship columns.
