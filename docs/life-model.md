# MLPL Life Model

`demos/life-plane/model.mlpl` is the pure model for the forthcoming native
Life-plane application. It contains no window, GPU, event, or Rust call.

## Grid and boundary contract

A grid is an owned record `{rows, columns, cells}`. `cells` is a row-major
binary `[rows*columns]` array, dimensions are integral from 1 through 256, and
all mutators return replacement records. `life_cell` defines every coordinate
outside the finite grid as dead; `life_set` rejects outside writes and values
other than zero or one. This dead-boundary policy is deliberate: the visual
plane does not connect its opposite edges.

Evolution zero-pads the board, uses `rotate` for all eight whole-array neighbor
shifts, applies Conway's vectorized B3/S23 rule, and extracts the original
finite area. The zero border prevents the toroidal wrap that plain `rotate`
would otherwise introduce. Only padding and extraction walk flat storage; the
automaton rule remains array arithmetic.

## Replacement presets

`life_preset(name, rows, columns, seed)` always starts from a new empty grid.
It supports:

- `empty`;
- still lifes `block` and `beehive`;
- oscillators `blinker` and `toad`;
- `glider`;
- the canonical 36-cell `glider-gun` (Gosper gun); and
- `random`, using MLPL's deterministic seeded random array.

Patterns are centered and reject grids too small to contain them. Selecting a
preset in the eventual UI will therefore replace, never merge with, the
current grid.

## Evidence and upstream comparison

`tests/test_life_model.mlpl` proves grid validation, dead outside cells, owned
replacement, block stability, blinker period two, four-generation glider
translation, canonical gun population and bounds, named preset populations,
and seeded-random repeatability.

The adjacent sw-MLPL Life research and evaluator fixtures supplied two useful
references: the whole-array `rotate` neighbor sum and canonical 36-cell gun.
Those examples use a torus, where edges wrap. This downstream model adds zero
padding to preserve the finite-plane contract. No sw-MLPL change is required.
