# Retained Native3D Scene Patches

Every interactive native demo now treats the live renderer as a retained
shadow scene: one initial `set_scene` snapshot (or an explicit recovery
resynchronization) is followed by bounded `patch_scene` transactions. Each
line has a stable non-negative semantic ID. The shared MLPL
`lib/native3d/retained.mlpl` differ compares those IDs and emits only changed,
added, or removed lines.
A patch carries `base_revision`, a greater `target_revision`, parallel upsert
arrays (`ids`, `[N,3]` starts and ends, `[N,4]` colors, and `[N]`
thicknesses), plus `[R]` removal IDs.

The host rejects duplicate IDs, an ID present in both operations, invalid
geometry/style values, non-advancing or stale revisions, unknown removals, and
patches exceeding 100,000 operations. It clones retained ID state, applies and
validates the candidate, and swaps it only after a complete valid scene can be
built. Failure therefore changes neither revision nor geometry. The host asks
the MLPL applet for a complete `set_scene` resynchronization after a rejected
live patch.

Life plane and torus grids retain their static lattice lines. MLPL compares the
old and new cell arrays and transmits four line upserts or removals only for
each changed cell. A five-cell glider initially sends 20 lines rather than the
plane's 102-line or torus's 1,620-line complete scene. Camera-only movement
continues to use `set_view`, and frame delivery remains single-flight. The
wireframe cube patches its twelve fixed edge IDs; tic-tac-toe assigns distinct
ID ranges to the grid, each cell's X/O/hover strokes, and the winning line;
both Model Atlas variants, disk usage, audio spectrum, and weight distribution
likewise keep their existing semantic IDs.

`set_view` advances view/status state without scene arrays. Its optional
`rotation_y_speed` field changes renderer animation state without pretending
that the line geometry changed. Pointer motion and wheel events are already
coalesced by the bounded native input queue before reaching MLPL.

MLPL owns stable-ID assignment, diff computation, Life state, topology, and
geometry. Rust owns bounded descriptor validation, atomic retained storage,
generic line-scene rebuilding, GPU resources, and resync signaling. The patch
protocol contains no cube, game, Life, filesystem, audio, model, or tensor
semantics.

This slice reduces MLPL allocation and Port transfer to O(changed lines).
The current Rust host still rebuilds its validated contiguous `LineScene` from
all retained lines after a successful patch, so host validation/upload work is
O(total retained lines). Incremental GPU-buffer mutation is not claimed.
Retained data and patch candidates are owned copies; there is no zero-copy
claim. winit/wgpu and the protocol are shared by macOS and Linux; interactive
visual evidence in this repository was collected on macOS.
