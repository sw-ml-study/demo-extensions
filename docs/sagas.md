# Saga Queue

Only one saga is active at a time. A later saga may be replanned when the
previous acceptance report exposes an upstream blocker. Steps are independently
reviewable and use red/green TDD; no step silently modifies `../sw-mlpl`.

The mandatory checklist in `AGENTS.md` applies to every step in every saga:
pre-commit tests, affected documentation, `.gitignore` audit, tracked-file
audit, named-file staging, a detailed commit, AgentRail completion metadata,
and a successful `git push origin main`. All work is directly on `main`; no
feature branches, PRs, `gh`, or GitHub Actions are used.

## Completed: extension-foundation

Purpose: establish the repository gate and prove the smallest public extension
boundary before graphics enters the dependency graph.

Executable AgentRail plan: `docs/extension-foundation-saga.md`.
Acceptance report: `docs/foundation-acceptance.md`.

1. `repository-tdd-scaffold` — Add the Rust workspace skeleton, MLPL source and test layout, root mlplunit configuration, tool-selection scripts, thin just recipes, and structural tests for the intended package layout.
2. `abi-v1-contract` — Specify minimal C-safe descriptor/value/error types and write Rust tests for layout, version negotiation, malformed descriptors, ownership, and panic containment before implementing them.
3. `hello-extension-registration` — Build an independently compiled hello cdylib and a safe test registry that loads, validates, registers, invokes, reports errors, retains its library lifetime, and deactivates it.
4. `manifest-and-module-facade` — Add deterministic manifest/platform resolution and an MLPL module facade, with traversal, mismatch, missing-artifact, duplicate-name, and diagnostic tests.
5. `foundation-acceptance-report` — Run Rust and MLPL gates, document REPL/script/compiled capability evidence and upstream gaps, decide static-provider parity, reconcile the next saga, and stop.

Acceptance: the headless hello path is independently built and contract-tested;
the user-facing namespace hides FFI; malformed or incompatible extensions fail
closed; and every unavailable upstream integration is documented rather than
mocked as complete.

## Completed: extension-sdk-arrays-handles

Purpose: make the proven ABI pleasant and safe for third-party Rust authors.

Executable AgentRail plan: `docs/extension-sdk-arrays-handles-saga.md`.

1. Factor descriptor registration behind dynamic and static provider guards,
   then run the identical hello contract against both without claiming the
   missing sw-MLPL compiler hook.
2. Add safe scalar/string/bytes conversions and error mapping.
3. Add function/type metadata and generate help/signature fixtures.
4. Add read-only dense numeric array views with shape/dtype/stride validation.
5. Add extension-scoped typed generational handles and lifecycle tests.
6. Add `#[mlpl_extension]`, `#[mlpl_fn]`, and `#[mlpl_type]` only where they
   remove stabilized boilerplate; publish an SDK acceptance example.

Acceptance: ordinary extension authors write no unsafe code, bulk `[N,3]`
arrays cross once, stale/wrong handles fail closed, and generated metadata is
identical to the hand-written contract.

## Completed: native-wireframe-cube

Purpose: make immediate, honest visual progress with an array-generated cube
and a generic native line renderer while upstream array/handle integration is
still pending.

1. Specify the MLPL-owned cube scene and generic renderer-neutral Rust schema.
2. Implement deterministic headless transform, projection, clipping, and line planning.
3. Add the opt-in macOS/Linux wgpu/winit window and interactive controls.
4. Connect deterministic scene generation to the native executable, document evidence, and publish acceptance.

Acceptance: MLPL bulk array operations define the cube; Rust contains no cube
semantics; headless evidence is authoritative; and the opt-in native window
rotates and adjusts dimensions, speed, color, and thickness without web
technology.

## Completed: native3d-pointer-camera

Purpose: add reusable pointer, wheel, bounded-frame, orbit-camera, and picking
contracts before building more application demos.

1. Specify pure input, orbit-camera, pick-ray, plane-hit, validation, and
   coalescing contracts with headless Rust TDD.
2. Connect winit pointer/wheel/frame events and MLPL-owned camera commands.
3. Add ordinary MLPL app, camera, picking, grid, and style helpers.
4. Migrate the cube to MLPL-owned drag orbit/tilt, wheel zoom, and drag pan.
5. Publish macOS smoke evidence, Linux limitations, bounded-event evidence,
   teardown results, and acceptance.

Acceptance: Rust translates platform input and renders a supplied camera but
contains no cube mouse map; MLPL owns camera transitions; high-rate events are
bounded/coalesced without reordering discrete clicks; headless evidence remains
authoritative.

## Completed: native3d-tic-tac-toe

Purpose: prove click picking and application state with a small complete game.

1. Build an MLPL board reducer, legal-move, result, and deterministic minimax
   suite with native mlplunit.
2. Render the board, X, and polygonal O marks through generic line arrays.
3. Add click-to-cell picking, X/O and first/second selection, restart, hover,
   result feedback, and clean close.
4. Launch the real native game and publish headless/live acceptance.

Acceptance: the user chooses X or O, turn order follows that choice, only empty
squares accept clicks, MLPL owns all rules and AI decisions, and Rust remains a
generic input/render service.

## Completed: native3d-tic-tac-toe-camera

Purpose: combine board clicks with the reusable 3D camera without ambiguous
release behavior.

1. Add MLPL-owned click/drag threshold arbitration, orbit/tilt, pan, and zoom.
2. Publish combined interaction acceptance and hand off to the Life-plane saga.

Acceptance: stationary clicks place marks, camera drags never place marks, all
cube-equivalent mouse controls work, and Rust remains application-neutral.

## Active: native3d-life-plane

Purpose: demonstrate array programming, editing, animation, and reusable 3D
camera interaction on a cellular grid.

1. `life-model` — implement deterministic MLPL Life evolution with an explicit
   finite dead-boundary policy; named empty, still-life, oscillator, glider,
   Gosper glider-gun, and deterministically seeded random presets; and
   mlplunit fixtures for evolution and placement.
2. `life-edit-and-controls` — add an initially empty grid, ray/plane cell
   picking, click/drag seeding, start/pause/step/clear, and deterministic
   click-versus-camera-drag arbitration. Publish a visible help legend for
   clear, each preset family, animation controls, editing, orbit/tilt, pan,
   and zoom.
3. `life-live-plane` — render the animated grid as generic bulk geometry,
   consume bounded frame events, and reuse orbit/tilt/zoom/pan without putting
   Life semantics in Rust.
4. `life-acceptance` — publish performance, memory, teardown, macOS visual
   evidence, Linux build evidence and limitations, and the MLPL/Rust ownership
   split.

Acceptance: users seed an empty grid before starting, animation remains MLPL
state evolution, the view is fully mouse-controlled, updates are bounded, and
the extension exposes no Life-specific primitive. Every keyboard binding is
visible in the native window, and choosing a preset has deterministic replace
semantics rather than silently merging with an existing grid.

The first implementation slice deliberately starts with the pure MLPL model.
That gives cell editing and frame animation one tested transition function and
keeps renderer performance choices out of the game rules.

## Queued: native3d-point-cloud

Purpose: prove a native macOS/Linux visualization using the public extension
path and no browser technology.

1. Add pure scene, camera, point attribute, and upload planning models with
   Rust TDD.
2. Add a wgpu renderer isolated from the ABI and a winit blocking event loop.
3. Expose viewer and point-cloud primitives through typed handles.
4. Add a deterministic MLPL bulk-array point-cloud demo and headless tests.
5. Record opt-in macOS/Linux smoke evidence, limitations, and lifecycle data.

Acceptance: the demo displays a bulk point cloud in a real native window,
headless tests remain authoritative, and no GPU/window types leak into the ABI.

## Queued: native3d-live-interaction

Purpose: support a persistent REPL-friendly viewer without compromising host
thread, callback, or unload safety.

1. Specify and test the host event-loop/thread contract.
2. Add bounded input/resize event polling and clean close behavior.
3. Add bulk geometry and attribute updates.
4. Add picking/pick-ray support tied to stable MLPL row IDs.
5. Stress deactivation with live handles, queued events, repeated REPL calls,
   and background activity.

Acceptance: update, select, close, and deactivate are deterministic; no stale
native resource remains callable; blocked unload explains exactly why.

## Queued: embedding-pca-explorer

Purpose: make array computation—not graphics—the center of the first flagship
application.

1. Add deterministic embedding/metadata fixtures and MLPL shape contracts.
2. Implement and test centering and PCA/projection in MLPL or document the
   narrow upstream primitive needed for a correct reference.
3. Map cluster, score, size, color, opacity, and IDs as parallel arrays.
4. Add click selection, boolean masks, and an array-debug presentation.
5. Verify REPL/script behavior and compiled parity when available.

Acceptance: the demo explains `[N,D] -> [N,3]`, all visual attributes are
array-driven, a selected point maps to its metadata row, and results are golden
and deterministic.

## Queued: rag-semantic-space

Purpose: turn the embedding viewer into a deterministic semantic retrieval
application while keeping retrieval logic in MLPL.

1. Add portable chunk and embedding fixtures with IDs and integrity checks.
2. Implement vectorized cosine scoring and golden top-k ranking.
3. Drive point emphasis and neighbor lines from score/index arrays.
4. Add query input and a scrollable selected-chunk details panel.
5. Compare later accelerated/indexed backends through the same MLPL contract.

Acceptance: golden queries, rankings, visual IDs, and metadata agree; the demo
does not require a network service or external vector database.

## Queued: geography-and-stars

Purpose: demonstrate that the extension primitives are reusable beyond ML.

1. Add world-capital coordinate transformations and point selection.
2. Add flat-buffer-plus-offset line strips for ragged country borders.
3. Add native overlay primitives needed for an MLPL-owned quiz/explore mode.
4. Add a star catalog with true-3D positions and array filtering.
5. Publish reuse, performance, and API-stability findings.

Acceptance: geography and quiz semantics remain MLPL-owned, both applications
reuse the same extension package, and ragged/bulk inputs are bounded and tested.
