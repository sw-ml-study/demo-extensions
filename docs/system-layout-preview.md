# SWTOS system-layout preview

`just system-layout-swtos` opens a native `wgpu`/`winit` window containing
the hash-pinned authoritative layout published by SWTOS. It exercises this
repository's MLPL consumer and generic renderer against real producer data.

The current SWTOS artifact contains one external W25Q32 `flash` space. Its
4,194,304-byte capacity is storage, not the system's RAM address space. The
default `Overview log` view uses normalized `log(byte length + 1)` bar heights
so all eight real regions remain visible despite only 372 bytes being used.
The `Physical` view retains proportional eight-byte-block geometry for honest
capacity comparison. Region ID 106 (`embedded-hello`) starts selected. Click
any box to select it.

The viewer is stationary by default. Click the visible `Rotate` control or
press `R` to toggle automatic rotation. Click the `Kind` or `Owner`
controls (or press `1` or `2`) to recolor every box. Press `H` repeatedly to
isolate legend categories in the current mode, and `C` to restore all regions. The colored legend
at the right changes with the selected mode. Click `Overview log` or `Physical`,
or press `V` to cycle views. Drag to orbit, Shift-drag to pan, use the wheel to
zoom, and press Escape to close the window.

## Run it

From this repository:

```sh
just system-layout-swtos
```

The older `just system-layout-preview` name remains as a compatibility alias.

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

The native preview currently shows overview boxes, a selected-item annotation,
and switchable color legends. Relationship edges, labels anchored directly to
each box, and adaptive block-level detail remain follow-up presentation work;
the fixture already carries the relationship columns and the shared MLPL
adapter already exposes deterministic block coordinates.

The queued block-layer view will use fine cubes only for occupied allocation
blocks (dozens for the current SWTOS artifact). Padding remains a distinct
kind. Free regions will not be expanded into cubes; each is represented by one
logarithmically scaled summary, keeping sparse layouts legible and bounded.
