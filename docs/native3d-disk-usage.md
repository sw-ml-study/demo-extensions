# Native 3D disk-usage explorer

Run `just disk-usage` to inspect this repository, or set the absolute
`DISK_USAGE_ROOT` environment variable. The demo captures one read-only
snapshot and never offers refresh, deletion, marking, moving, renaming, or any
write operation.

The Rust host performs only `read_dir` and `symlink_metadata` calls. It never
opens file contents, never follows symlinks, and stops at 256 retained entries
or depth 16. Excluded and inaccessible entry counts remain visible;
excluded bytes are a known-byte lower bound because the scanner does not
exceed its budget merely to estimate omitted subtrees.

MLPL owns recursive aggregation, directory-first ordering with largest-first
ordering inside each kind, thresholds,
breadcrumb navigation, selection, camera behavior, labels, and scene
commands. Rust owns the generic bounded snapshot, window, input normalization,
and rendering. The scene exposes sixteen bounded display slots per level;
status text reports the selected path and whether it is a directory or file.

The app retains a compact, sorted child view in MLPL state. It recomputes that
view only after entering/leaving a directory or changing the threshold.
Up/down selection emits a stable-ID patch for only the old and new bars;
orbit, pan, zoom, and status changes use view-only commands. The initial scene
is never resent during ordinary interaction. This shadow-scene/diff behavior
is required for interactive performance.

Controls are shown in the window: up/down selects; right or Enter drills into
a selected directory; left, P, Backspace, or Delete returns to the parent;
click selects; T toggles the 1 MiB threshold; R resets the camera; and the
standard mouse controls orbit, pan, and zoom. Green shades identify
directories, blue/purple/orange identify files, and yellow identifies the
selection. A file cannot be descended into, and the status says so explicitly.

The same `std::fs`, `winit`, and `wgpu` path is used on macOS and Linux. An
interactive smoke requires a desktop GPU/session; headless MLPL and Rust tests
are the portable acceptance evidence.

`fs_walk` currently has no entry limit, so this app deliberately uses its own
generic bounded host snapshot rather than invoking an unbounded language walk.
