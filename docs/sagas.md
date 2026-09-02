# Saga Queue

Only one saga is active at a time. A later saga may be replanned when the
previous acceptance report exposes an upstream blocker. Steps are independently
reviewable and use red/green TDD; no step silently modifies `../sw-mlpl`.

Current status (2026-09-02): `native3d-point-cloud` has delivered its initial
renderer-neutral contract and headless rendering steps. The
`native3d-retained-scene` saga completed through step 015, including Model Atlas,
disk usage, audio spectrum, weight distribution, the Yew microscope, and
repository-wide retained-scene migration plus documentation reconciliation.
Embedding/PCA remains the recommended successor after the point-cloud saga.

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

## Completed: native3d-life-plane

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

## Completed: native3d-retained-scene

Purpose: keep native interaction responsive when MLPL geometry work is slower
than display refresh, then introduce generic shadow-scene patches.

1. Add acknowledgement-driven single-flight frames so stale animation events
   cannot starve later key or pointer input.
2. Add a toroidal Life demonstration with two-axis wrap-around, curved MLPL
   geometry, surface picking, and the existing controls.
3. Add versioned ID-addressed atomic add/update/remove patches and use them for
   changed Life cells instead of complete scene replacement.

All steps are complete. The final repository-wide pass also migrated cube,
tic-tac-toe, real-file Model Atlas, disk usage, audio, and weight-distribution
interaction to the same retained invariant.

Acceptance: at most one frame is outstanding across the Port, discrete input
stays ordered, malformed or stale patches fail closed, and retained Rust scene
objects contain no Life rules.

## Delivered: native3d-model-atlas

Purpose: connect bounded model-file analysis from `demo-ml-utils` to this
repository's generic native renderer without loading a whole model file or
tensor payload into memory.

1. Define a range-read, optionally multi-pass scanner that retains only capped
   catalog/summary IR and fetches selected details on demand. The generic
   scanner and sparse-memory evidence are complete; format-derived interchange
   fixtures follow in the next step.
2. Pin a versioned renderer-neutral interchange compatible with tensor-city
   Safetensors/GGUF output and explicit provenance. The derived cross-format
   golden, bounded tagged transport, and deterministic layout validation are
   complete.
3. Render interactive tensor buildings, districts, labels, selection,
   filtering, camera controls, and bounded level of detail. **Complete:** the
   derived cross-format fixture now runs through `just model-atlas` with
   MLPL-owned scene semantics and retained stable-ID patches.
4. Add architecture metadata plus visibly labeled tensor-name inference for
   embeddings, attention, MLP, normalization, MoE, Mamba/SSM, and output heads.
   **Complete:** the UI distinguishes GGUF `[METADATA]` from selected tensor
   `[HEURISTIC]` roles and fails ambiguous/adversarial names to `UNKNOWN`.
5. Add on-demand bounded statistics, histogram/surface, and quantization-error
   detail views using stable-ID scene patches. **Complete:** checked-in
   Safetensors/GGUF fixtures use `demo-ml-utils` compatible schemas, enforce
   4/4/4 caps, patch only selected/detail IDs, and visibly reject unsupported
   payload decoding.
6. Open on a confined real-file picker, reuse the bounded `demo-ml-utils`
   Safetensors catalog, render a capped tensor view, and return with M.
   **Delivered:** `just model-atlas-file` starts
   at the picker and never reads tensor payloads. GGUF remains a later format
   addition.

Acceptance: total model size does not determine resident payload memory; every
range/pass/cache budget fails closed; metadata and heuristic inference remain
visibly distinct; Rust contains no model semantics; and macOS/Linux use the
same native winit/wgpu path.

## Delivered: native3d-audio-spectrum-player

Purpose: make compressed-audio processing visible as a native, normal-speed
3D stereo spectrum rather than treating decoding as a silent batch task.

1. Reuse bounded MP3/Ogg discovery and decode contracts from
   `demo-file-processing`, with explicit frame/ring-buffer budgets.
2. Expose generic native audio output and timestamp primitives only where the
   existing extension boundary lacks them; keep player state in MLPL.
3. Compute deterministic windowed stereo spectra and MLPL mappings for bass,
   mid-range, and high bands, with channel/color/scale legends.
4. Render a native 3D equalizer with play/pause, seek, orbit/pan/zoom, file
   menu, and an option to mute or play synchronized audio.
5. Add headless decoder/spectrum/timing tests plus opt-in macOS/Linux audio and
   window smoke evidence.

Acceptance: decoding and visualization remain bounded while playing at normal
speed; left/right channels and frequency units are unambiguous; muted mode is
fully useful; audio synchronization has measured drift bounds; and neither
the Rust extension nor renderer encodes application-specific equalizer rules.

## Delivered: native3d-disk-usage-explorer

Purpose: demonstrate a read-only native disk-usage explorer inspired by
`dua-cli`, using MLPL aggregation and the generic renderer to explain which
directories and files consume a selected tree's space.

1. Define a confined, read-only scan contract over one host-selected root.
   Record relative path, entry kind, file size, parent identity, scan errors,
   and explicit entry/depth/output budgets; never read file contents.
2. Aggregate direct and recursive byte totals by directory in MLPL, preserve
   stable path IDs, sort deterministically by descending bytes then path, and
   account visibly for inaccessible or budget-excluded entries.
3. Present an initial root picker/confirmation view followed by a native 3D
   treemap or nested-block view. Area or footprint represents recursive bytes;
   height may represent direct bytes or depth only when the legend makes that
   mapping explicit. Use logarithmic scaling only where labeled with units.
4. Add click drill-down, parent/back navigation, largest-first filters,
   minimum-size thresholds, orbit/pan/zoom, selection details, and a persistent
   breadcrumb. Retain the completed snapshot; this demo has no recalculate,
   refresh, mark, delete, move, or write action.
5. Add deterministic synthetic-tree mlplunit tests, confined-filesystem Rust
   tests, sparse/large-tree memory evidence, macOS/Linux interactive smoke
   evidence, and documentation comparing the deliberately narrower behavior
   with full disk-management tools.

Safety and ownership: every filesystem operation is metadata-only and confined
to the selected root. Symlink traversal policy is explicit and escape-safe.
Rust supplies generic sandboxed metadata, window, event, and rendering
primitives; MLPL owns aggregation, ranking, navigation, labels, and visual
mapping. The application exposes no mutation command, including hidden keys.

Acceptance: users can identify the largest directories/files and navigate the
captured hierarchy; totals and unknown/excluded bytes reconcile under fixed
budgets; file contents are never opened; the snapshot does not change unless
the application is closed and deliberately launched again; and tests prove no
remove/write/rename primitive is reachable from the demo.

Delivered behavior uses a bounded metadata-only breadth-first snapshot, an
MLPL-cached sixteen-item child view, directory/file/selection palettes,
four-way keyboard navigation plus click selection, and stable-ID scene diffs.
The scalar-shape limitation affecting recursive-total status formatting is
recorded in `docs/sw-mlpl-blockers.md` rather than hidden by a correctness
claim.

## Delivered evidence: native3d-audio-spectrum-player

The current slice supplies a confined bounded MP3/Ogg picker, incremental
MP3/Ogg-Vorbis decoding, MLPL-owned stereo frequency analysis, a mirrored
radial bass/mid/high display, retained 16-spoke patches, normal-speed
coalesced visualization, independently bounded decode-ahead, and synchronized
default-device audio. MP3 and Ogg/Vorbis playback were interactively confirmed
on macOS after correcting chunk-boundary timing distortion. Linux uses the same
CPAL/winit/wgpu source with ALSA development/runtime requirements documented.

## Delivered evidence: native3d-weight-distribution-explorer

Purpose: inspect real model weight distributions through bounded reads and
mergeable statistics, then render MLPL-owned histogram, surface, channel, and
quantization-detail views without loading a tensor or model file wholesale.

The first slice is intentionally limited to decoders already available through
the downstream model utilities: supported byte-aligned Safetensors numeric
types, GGUF I8/I16, and GGUF Q8_0. Unsupported F16/BF16/F32 paths and Q4/Q5/K
quantizers fail closed with a visible reason until their shared decoder
contracts exist. The delivered ownership matrix distinguishes requirements
owned by `demo-ml-utils` from genuine `sw-mlpl` host gaps in
`docs/weight-distribution-blockers.md`; no language change is required for the
initial supported-dtype slice.

**Delivered first slice:** `just weight-distribution` now offers a real
Safetensors/GGUF picker, tensor selection, capped aligned integer/Q8_0 samples,
an eight-bin logarithmic-height histogram, visible units/legends/statistics,
camera controls, stable-ID selection patches, and explicit fail-closed decoder
reasons. The exact shared-library and host ownership matrix is recorded in
`docs/weight-distribution-blockers.md`.

## Partially delivered, not active: native3d-life-surfaces

Purpose: reuse the Life application on closed 3D surfaces after the finite
plane is accepted.

1. Separate neighbor topology from surface geometry in the MLPL library.
2. Add a toroidal grid with wrap-around in both axes and map cells onto a
   native 3D torus (donut). The first torus slice is delivered by
   `just life-torus`.
3. Add a spherical mapping with an explicit pole/seam adjacency policy rather
   than pretending a rectangular grid wraps uniformly at the poles.
4. Reuse editing, animation, presets, picking, orbit/pan/zoom, and generic bulk
   rendering; publish topology fixtures and macOS/Linux evidence.

Acceptance: the same MLPL Life rule runs against documented plane, torus, and
sphere neighbor policies; seams and poles have golden tests; Rust contains no
Life rule or preset; and surface selection changes topology and projection
explicitly rather than accidentally inheriting plane boundaries.

The plane and torus are delivered. Sphere topology/projection is future scope
and has no active AgentRail step.

## Active: native3d-point-cloud

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

## Partially delivered, remaining work not active: native3d-live-interaction

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

The interpreted applets already provide bounded/coalesced input, resize,
stable-ID updates, picking, and clean close. Persistent REPL handles, compiled
provider parity, and quiescent dynamic unload remain future extension-boundary
work.

## Future candidate after point cloud: embedding-pca-explorer

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

## Future candidate: rag-semantic-space

Purpose: turn the embedding viewer into a deterministic semantic retrieval
application while keeping retrieval logic in MLPL.

1. Add portable chunk and embedding fixtures with IDs and integrity checks.
2. Implement vectorized cosine scoring and golden top-k ranking.
3. Drive point emphasis and neighbor lines from score/index arrays.
4. Add query input and a scrollable selected-chunk details panel.
5. Compare later accelerated/indexed backends through the same MLPL contract.

Acceptance: golden queries, rankings, visual IDs, and metadata agree; the demo
does not require a network service or external vector database.

## Future candidate: geography-and-stars

Purpose: demonstrate that the extension primitives are reusable beyond ML.

1. Add world-capital coordinate transformations and point selection.
2. Add flat-buffer-plus-offset line strips for ragged country borders.
3. Add native overlay primitives needed for an MLPL-owned quiz/explore mode.
4. Add a star catalog with true-3D positions and array filtering.
5. Publish reuse, performance, and API-stability findings.

Acceptance: geography and quiz semantics remain MLPL-owned, both applications
reuse the same extension package, and ragged/bulk inputs are bounded and tested.
