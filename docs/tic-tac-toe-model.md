# MLPL Tic-Tac-Toe Model

Status: pure rules and deterministic AI, 2026-08-16

`demos/tic-tac-toe/model.mlpl` is a renderer-independent game model written
entirely in ordinary MLPL. Rust has no board, turn, winning-line, or AI logic.

## Representation and validation

The board is a dense row-major `[9]` numeric array. `0` is empty, `1` is X,
and `-1` is O. Validation checks shape, cell values, reachable X/O counts, and
bounded reachable X/O count differences. Mark and turn-order choices are
independent for this demo: either X or O may make the first board move.

Move placement rejects an invalid board, nonintegral or out-of-range cell,
occupied cell, terminal game, invalid mark, and out-of-turn mark. Outcome
records use `playing`, `won`, or `draw` plus a numeric winner.

## Deterministic minimax

`u:ttt_best_move(board,ai_mark)` explores terminal outcomes with depth-aware
scores, uses alpha-beta pruning, and scans cells from 0 through 8. Equal scores
therefore select the lowest legal cell, making demonstrations and tests
reproducible. The implementation is deliberately small and educational rather
than a native optimized game primitive.

The native mlplunit suite proves valid/invalid boards, legal and occupied
moves, row/column wins, draw and in-progress outcomes, X/O plus first/second
setup, immediate wins, required blocks, and deterministic empty-board choice.
The later rendering and interaction steps consume this model without changing
its rules.

## Generic line scene

`scene.mlpl` maps model state to the reusable native3d bulk-line contract. The
empty board is four grid segments. Each X is two diagonal segments and each O
is an eight-segment polygon. An empty hovered cell gets a four-segment green
outline; a win gets a thick gold line through its winning cells. Hover is
suppressed for occupied cells and terminal boards.

Every line owns two `[x,y,z]` endpoints. The resulting arrays have shapes
`[2M,3]` positions, `[M,2]` edges, `[M,4]` colors, and parallel `[M]`
thickness/ID vectors. IDs are deterministic `0..M-1`, rotation is zero, and
all arrays are owned MLPL values. No tic-tac-toe renderer or mark primitive was
added to Rust. Native mlplunit pins empty, marked, hovered, occupied-hover, and
winning geometry plus deterministic output.

## Live native game

`just tic-tac-toe` selects the tic-tac-toe MLPL applet in the same macOS/Linux
winit/wgpu host used by the cube. The host now accepts bounded variable
`[N,3]`/`[M,2]` line arrays with parallel per-edge colors and thicknesses; this
is generic renderer capability, not a game API.

MLPL converts physical-pixel pointer coordinates to a world pick ray,
intersects the XZ board plane, and maps the hit to a row-major cell. Only an
empty cell on the human turn is accepted. X/O changes the human mark, 1/2
changes turn order, R restarts with the selected choices, and Escape closes.
The visible overlay states these controls and the selected mark/order; hover
and a gold winning line provide graphical feedback.

The same reducer owns camera gesture arbitration. A stationary left
press/release is a board click. Movement beyond four physical pixels converts
the gesture to left-drag orbit/tilt or Shift-left/middle-drag pan and suppresses
placement on release. Wheel input zooms. Hover resumes after the drag ends.
Rust still supplies only normalized pointer/button/modifier/wheel records.

Exhaustive alpha-beta minimax remains the deterministic reference tested by
mlplunit. The live worker uses a bounded-loop perfect-play policy—win, block,
center, opposite corner, corner, then edge—because the host intentionally has
a bounded worker stack and recursive interpreter frames overflowed it. Both
implementations are MLPL; Rust contains neither policy nor game rules.

The Rust integration test runs the real applet/Port path without a display,
delivers a normalized center click, and verifies that the initial four-line
grid becomes a variable styled scene containing both the human and AI marks.
It also reproduces the formerly failing O-first sequence—press O, then click
the center—and proves the applet remains alive with an O and an X rendered.
Terminal-board drag/release and off-board `-1` picks are explicit no-ops, pinned
after interactive testing exposed an eager out-of-bounds board lookup.
The interactive smoke remains opt-in and uses identical source.
