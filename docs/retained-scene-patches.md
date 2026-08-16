# Retained Native3D Scene Patches

The live native renderer accepts a complete `set_scene` snapshot followed by
bounded `patch_scene` transactions. Each line has a stable non-negative ID.
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
continues to use `set_view`, and frame delivery remains single-flight.

MLPL owns stable-ID assignment, diff computation, Life state, topology, and
geometry. Rust owns bounded descriptor validation, atomic retained storage,
generic line-scene rebuilding, GPU resources, and resync signaling. The patch
protocol contains no Life, torus, model, or tensor semantics and is intended
to support Model Atlas buildings and selection/detail overlays next.

This slice reduces MLPL allocation and Port transfer to O(changed lines).
The current Rust host still rebuilds its validated contiguous `LineScene` from
all retained lines after a successful patch, so host validation/upload work is
O(total retained lines). Incremental GPU-buffer mutation is not claimed.
Retained data and patch candidates are owned copies; there is no zero-copy
claim. winit/wgpu and the protocol are shared by macOS and Linux; interactive
visual evidence in this repository was collected on macOS.
