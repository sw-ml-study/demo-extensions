# Native Tic-Tac-Toe Acceptance

Status date: 2026-08-16

| Capability | Status | Evidence | Limitation |
|---|---|---|---|
| Board validation and legal moves | Proven | `test_tic_tac_toe_model.mlpl` | Numeric `[9]` teaching representation |
| Win, draw, and in-progress outcomes | Proven | row, column, draw, and live fixtures | Winning-line geometry is the result display |
| Deterministic reference minimax | Proven | immediate win, forced block, stable opening | Recursive reference is headless-only |
| Bounded live perfect play | Proven | MLPL win/block/center/corner/edge policy tests | Separate implementation for bounded worker stack |
| Independent X/O and first/second choices | Proven live | MLPL tests and O-first worker/Port regression | Keyboard selection in the PoC |
| Click-to-cell picking | Proven | center-ray MLPL test and live center click | Physical-pixel coordinates |
| Click/drag arbitration | Proven | four-pixel threshold MLPL and worker/Port tests | Physical-pixel threshold |
| Orbit, tilt, pan, zoom | Proven live | shared camera reducer plus real applet commands | Camera semantics remain MLPL-owned |
| Legal empty-cell interaction | Proven | occupied/no-turn/terminal/off-board no-op tests | No animated transition |
| Grid, X/O, hover, win feedback | Proven | deterministic bulk-shape mlplunit fixtures | Line graphics only |
| Variable per-line native rendering | Proven | Rust headless style planning and live command parsing | Owned copies; no zero-copy claim |
| Clean close | Proven | Escape/close worker teardown and macOS smoke | Local interpreted applet |
| macOS native window | Proven | repeated `just tic-tac-toe` runs | Manual visual evidence |
| Linux portability | Design/build supported | shared winit/wgpu/WGSL and MLPL source | Not visually tested on this Mac |
| Compiled MLPL binary | Blocked upstream | `sw-mlpl-blockers.md` | Compiler/provider startup parity |

## Regression evidence

Two failures discovered through interactive use are pinned. Selecting O and
then clicking center formerly returned a move error because the model assumed
X must open; mark and turn-order choices are now independent. Dragging after a
tied game formerly indexed the board with the off-board `-1` sentinel because
an eager condition evaluated before its bounds guard. Terminal and off-board
pointer releases are now explicit no-ops.

Stationary left release places a mark; left drag orbits/tilts, Shift-left or
middle drag pans, and wheel input zooms. Once movement crosses four pixels, the
release is never interpreted as a board click. This arbitration is deterministic
and covered both in native mlplunit and through the real worker/Port applet.

## Ownership and next demo

MLPL owns every game-specific rule, board transition, AI choice, pointer-cell
mapping, choice binding, hover decision, and generated line array. Rust owns
only bounded normalized events, validated generic array/style ingestion,
projection, GPU drawing, the native window, and teardown.

No new sw-MLPL primitive is required for the queued Life-plane saga. It can
reuse the camera, pick-ray/plane, grid, bounded frame events, and generic bulk
line path. Efficient filled cells or instances may justify a generic extension
primitive after profiling; that is downstream renderer scope, not presently a
language blocker.
