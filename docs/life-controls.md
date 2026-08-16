# MLPL Life Controls

`demos/life-plane/controls.mlpl` owns the interactive application state and
reduces normalized input records without calling a window or renderer. The
default is an empty 40×40 finite grid, paused at generation zero.

## Mouse arbitration

- A stationary left click toggles the picked cell.
- Control-left press and drag paints live cells. Painting never moves the
  camera and is idempotent when a pointer revisits a cell.
- Plain left movement beyond four physical pixels becomes orbit/tilt and its
  release never edits a cell.
- Shift-left or middle drag pans. The wheel zooms.

Picking uses the shared MLPL camera ray and intersects the horizontal Life
plane. MLPL converts that world point into a finite row-major cell. Outside
hits are harmless.

## Keyboard and animation

Space starts or pauses, N advances one generation, C clears, and plus/minus
adjust generation speed from 1 through 30 steps per second. A frame event can
advance at most one generation, preserving a bounded reducer even after a long
host delay.

B/H choose block/beehive still lifes, I/T choose blinker/toad oscillators, G
chooses a glider, U chooses the Gosper gun, and R chooses a deterministic
random replacement while advancing its seed. Every preset pauses, replaces
the full grid, and resets the generation counter. Escape/close requests clean
teardown.

`life_help()` is the exact multiline legend intended for the native window.
Its full text is pinned by mlplunit so a binding cannot silently disappear
from the UI.

## Native plane

`just life-3d` runs the portable `scripts/run-life-3d` entry point. The MLPL
scene emits 41 horizontal and 41 vertical generic grid lines plus four line
segments for each live or hovered cell. This baseline intentionally reuses the
established bulk-line API; no Life-specific or filled-cell primitive was added
without profiling evidence.

The worker/Port test sends preset, run, frame, wheel, and close events through
the actual applet. Geometry changes send complete owned scenes, camera changes
send retained-scene `set_view` diffs, and nonvisual transitions send nothing.
This prevents paused frames and idle mouse motion from queueing full 1,600-cell
rebuilds. The native smoke run remains the visual evidence.

## Ownership and future topology

Life rules, state, picking decisions, gesture arbitration, presets, timing,
and camera transitions are MLPL-owned. Rust will continue to provide generic
normalized events and bulk geometry rendering.

A queued follow-on will separate topology from projection: the current plane
keeps dead edges, a torus wraps both grid axes and renders on a donut, and a
sphere receives a tested seam/pole adjacency policy. That work follows plane
acceptance so closed-surface topology does not obscure the base interaction
contract.
