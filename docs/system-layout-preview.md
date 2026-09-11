# Synthetic system-layout preview

`just system-layout-preview` opens a native `wgpu`/`winit` window containing
the hash-pinned synthetic layout published by sw-MLPL. It lets the consumer
and renderer evolve while the authoritative SWTOS, MLOS, and MesaOS producer
artifacts are still pending.

The three vertical groups are the fixture's `flash`, `ebr`, and `sysram`
spaces. Box height represents byte length in eight-byte blocks. Region ID 8
starts selected, and its name and owner appear in the annotation. Click any
box to select it.

The viewer is stationary by default. Click the visible `Rotate` control or
press `R` to toggle automatic rotation. Click the `Kind`, `Owner`, or `Space`
controls (or press `1`, `2`, or `3`) to recolor every box. The colored legend
at the right changes with the selected mode. Drag to orbit, Shift-drag to pan,
use the wheel to zoom, and press Escape to close the window.

## Run it

From this repository:

```sh
just system-layout-preview
```

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

The source is sw-MLPL revision `baf015ff`, file
`examples/viz/storage-layout.json`, whose published SHA-256 is
`b5a328a623fd7b8378bf47b7014318525334495a9911354539a007f0fbfa657b`.
The repository keeps a whitespace-normalized vendored copy at
[`fixtures/sw-mlpl-storage-layout-baf015ff.json`](../fixtures/sw-mlpl-storage-layout-baf015ff.json),
SHA-256 `be71a69ec5e4e77a41e0c3d45a1bbf08dc288a2161ca10e5970d227dcdf4e87f`.
The launcher refuses to run if that copy changes unnoticed.

This fixture is development evidence, not SWTOS acceptance evidence. Its
provenance intentionally says it is a sample, and it must not be reported as
an emitted OS artifact. The active AgentRail step remains open until a
committed producer artifact is available. MLOS and MesaOS will use the same
producer-neutral columnar contract in later steps.

The native preview currently shows overview boxes, a selected-item annotation,
and switchable color legends. Relationship edges, labels anchored directly to
each box, and adaptive block-level detail remain follow-up presentation work;
the fixture already carries the relationship columns and the shared MLPL
adapter already exposes deterministic block coordinates.
