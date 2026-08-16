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
