# MLPL Tic-Tac-Toe Model

Status: pure rules and deterministic AI, 2026-08-16

`demos/tic-tac-toe/model.mlpl` is a renderer-independent game model written
entirely in ordinary MLPL. Rust has no board, turn, winning-line, or AI logic.

## Representation and validation

The board is a dense row-major `[9]` numeric array. `0` is empty, `1` is X,
and `-1` is O. Validation checks shape, cell values, reachable X/O counts, and
winner/count consistency. X always makes the first board move, independent of
whether the human chooses X/O or first/second.

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
