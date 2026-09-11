# Synthetic system-layout preview

`just system-layout-preview` opens a native `wgpu`/`winit` window containing
the hash-pinned synthetic layout published by sw-MLPL. It lets the consumer
and renderer evolve while the authoritative SWTOS, MLOS, and MesaOS producer
artifacts are still pending.

The three vertical groups are the fixture's `flash`, `ebr`, and `sysram`
spaces. Color identifies `region_kind`; box height represents byte length in
eight-byte blocks. Region ID 8 starts selected. Drag to orbit, use the wheel
to zoom, and press Escape to close the window.

## Run it

From this repository:

```sh
just system-layout-preview
```

There is no setup or generated data to retain. The launcher selects the
adjacent development MLPL binary (or the absolute `MLPL` override), evaluates
[`scene.mlpl`](../demos/system-layout-preview/scene.mlpl), writes its generic
box-scene JSON to a temporary directory, and launches the native3d window.

MLPL owns JSON parsing, kind-to-color classification, space placement,
block geometry, stable IDs, and scene construction. Rust owns only generic
bulk box validation, rendering, camera input, and selection highlighting.

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

The native preview currently shows overview boxes. Relationship edges,
labels, and adaptive block-level detail remain follow-up presentation work;
the fixture already carries the relationship columns and the shared MLPL
adapter already exposes deterministic block coordinates.
