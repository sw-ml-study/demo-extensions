# sw-MLPL Blockers for Interactive Native 3D

Status date: 2026-08-16

The native wgpu/winit renderer and MLPL-generated cube scene work today.
sw-MLPL commits `5c695fe1`, `03c7559b`, `797d910f`, and `f8585846` have now
shipped dense arrays in both directions, opaque native handles, and nested
structured record returns, parked-main UI launch, handler dispatch, and bounded
Port delivery. This repository now proves the complete local interpreted loop.
This repository does not modify `../sw-mlpl`.

Pointer, wheel, frame, orbit-camera, and pick-ray work is downstream extension
and MLPL-library work. Existing owned records, arrays, handler dispatch, and
bounded Ports can express it; no additional sw-MLPL language primitive is
currently required. Compiled application parity remains the upstream gap.

## Required host primitives

### Typed persistent native handles — closed for interpreted static providers

sw-MLPL must transport extension-scoped, type-tagged generational handles as
language values. A viewer returned by `native3d:open(...)` must remain valid
across later MLPL evaluations and reject stale, forged, wrong-type, and
cross-extension handles. Deactivation must finalize viewers deterministically.

Downstream acceptance: an MLPL test creates, queries, closes, and then fails to
reuse a viewer; negative tests cover every invalid-handle class and repeated
close.

### Bounded event polling and host event-loop ownership — closed locally

The extension needs a non-callback primitive such as
`native3d:poll_events(viewer, limit)` that returns ordered key, pointer, resize,
and close records. The host must specify macOS main-thread ownership, Linux
event-loop behavior, queue bounds, backpressure, cancellation, REPL lifetime,
and shutdown. Polling is preferred for the PoC because it avoids language
reentrancy from native callbacks.

Downstream acceptance: MLPL opens a viewer, polls bounded batches, handles
resize and close, and exits cleanly under repeated open/poll/close cycles.
Overflow and provider deactivation have deterministic results.

### Bulk arrays through the public C-provider boundary — closed for interpreted static providers

The host C adapter must accept dense numeric arrays with dtype, rank, shape,
strides, ownership, and call lifetime. The PoC needs `[N,3]` positions and
`[M,2]` edges plus parallel color, thickness, and ID arrays. Contiguous,
read-only, call-scoped borrowing is sufficient initially; unsupported layouts
must fail with useful diagnostics.

Downstream acceptance: one MLPL call updates a complete line scene in bulk.
Tests cover wrong dtype/rank/shape, invalid indices, overflow, non-contiguous
storage, expired storage, and explicit copy-versus-borrow evidence.

### Generic viewer calls exposed to MLPL — interpreted live slice proven

The public extension surface must be able to register and invoke these generic
operations:

- `open(config) -> ViewerHandle`
- `poll_events(viewer, limit) -> [Event]`
- `set_lines(viewer, positions, edges, colors, thicknesses, ids)`
- `render(viewer, time_or_transform_state)` and `present(viewer)`
- `drawable_size(viewer)` and monotonic-time access
- `close(viewer)`

Rust translates winit events and manages wgpu resources. MLPL owns the
application loop and maps input to width, height, length, rotation speed,
color, thickness, pause, and reset. A Rust-owned cube key map is not accepted
as completion.

Downstream acceptance: native mlplunit exercises the public functions through
the real host adapter, not only the downstream Rust registry.

Current evidence: the headless `_native3d` provider implements create, bulk
set-lines, size/state records, explicit render state, and close through the
actual interpreter in `native3d_provider.rs`. The live parked-main host now
delivers normalized key, pointer, wheel, coalesced frame, resize, and close
records and consumes validated MLPL-owned camera commands. Reusable MLPL camera
reduction and cube mappings are now proven downstream by native mlplunit and
the live worker/Port test; they require no upstream change.

### Interpreted and compiled parity

The same MLPL application must resolve the provider and behave consistently in
the REPL, interpreted scripts, and eventually compiled binaries. The compiler
needs an extension registration/startup hook and artifact packaging or static
provider linkage without changing MLPL source.

Downstream acceptance: the same cube source runs interpreted and compiled,
with equivalent events, scene updates, diagnostics, and close behavior.

## Current interpreted result

`just cube-3d` runs winit/wgpu on the main thread and sw-MLPL on a worker.
Generic owned events flow to `controls.mlpl`; complete owned scene commands
flow back. Headless protocol tests and native mlplunit prove the same logic.

## Upstream handoff order

1. Add provider startup, linkage/package, and call parity to compiled output.
2. Add dynamic package loading and a demonstrated quiescent unload protocol.

There is no remaining sw-MLPL blocker for the local interpreted interactive
PoC. Compiler and deployment parity remain explicit later capabilities.

## Model-picker filesystem sandbox — parity gap with downstream workaround

The file-backed Model Atlas can use `fs_walk`, `file_size`, and bounded
`read_bytes` in ordinary sw-MLPL script mode, but the parked-main applet entry
point cannot currently configure their sandbox. `run_applet_with_host(source,
host)` constructs `Environment::new()` internally and leaves `fs_root` unset;
there is no public applet configuration or root parameter. Consequently an
MLPL applet receives `err("...no filesystem sandbox on this surface...")` for
directory discovery and file reads, even when the user explicitly selected a
root in the native launcher.

Required upstream contract: add a configured parked-main entry point, for
example `run_applet_with_host_config(source, {fs_root}, host)`, that
canonicalizes one explicit root before the worker starts and gives the worker
the same contained `fs_walk`/`file_size`/`read_bytes` behavior as script mode.
It must reject absent, non-directory, and non-canonical roots; prevent `..` and
symlink escape; and preserve the existing no-filesystem default for callers
that do not opt in. The root is host policy, not an MLPL-selected unrestricted
path.

Downstream status: this repository now uses a small adapter over sw-MLPL's
public `Environment`, `register_port`, and filesystem-root field. It
canonicalizes one host-selected directory before starting the worker while
preserving the ordinary applet helper's no-filesystem default. Headless tests
prove confined discovery, selection, bounded header analysis, return to the
menu, and denial on the unconfigured surface. This unblocks the interpreted
demo without modifying sw-MLPL, but the configured parked-main entry point is
still needed for first-class host parity and to remove the downstream adapter.

## Confined filesystem modification times — interpreter shipped

sw-MLPL commit `0f4d0e32` shipped `file_metadata(path) ->
ok({kind,size,modified_unix_ms})` for the interpreter. It uses exact UTC Unix
milliseconds, returns an error when modification time is unavailable, and
shares the `file_size` configured-root and symlink-escape rules. This removes
the interpreted Model Atlas date blocker.

Primary owner: `../sw-mlpl`; the interpreter contract above is complete and
documented. Remaining upstream work is `file-metadata-compiler-parity`, which
must lower the same Result record and confinement behavior to compiled Rust.
There is intentionally no upstream date-formatting builtin: UTC/local
formatting, sorting, and presentation are MLPL application semantics.

Downstream adopter: `../demo-file-processing` should now demonstrate bounded
metadata scans, date sorting/formatting, unavailable-time handling, and
macOS/Linux fixtures. This repository can consume the same API immediately in
its interpreted model picker. Neither downstream repo needs or should add a
competing native filesystem API.

Downstream acceptance here: menu rows show an unambiguous size and modification
timestamp for each candidate, sorting remains deterministic when timestamps
tie or are unavailable, displayed timezone/format is labeled, and headless
tests pin exact epoch values without depending on wall-clock time.

Separate remaining applet gap: `run_applet_with_host` still cannot accept a
configured filesystem root, so this repository's public-`Environment` adapter
remains necessary even though `file_metadata` itself has shipped.

## Derived scalar reuse from array/record computations — open

The disk-usage MLPL implementation exposed an interpreter defect while
formatting or conditionally rendering values derived from array-backed record
fields. A scalar child count or recursive total can pass arithmetic and
assertion checks, then fail when reused by `to_json`, string construction, or
conditional geometry with `array error: shape mismatch: 1 vs 1 elements`.
The dimensions and element counts reported by the error are identical.

This repository works around the defect by retaining a fixed-capacity compact
child view, storing a separately serialized count, limiting geometry to
sixteen stable slots, and omitting the affected recursive-total value from the
selected status. The workaround keeps this demo usable but is not a general
language solution.

Owner: `../sw-mlpl`. Required fix: preserve scalar rank/shape consistently
when extracting a single value from arrays nested in records, including reuse
across conditionals, `to_json`, and function boundaries. Add a minimal
regression that constructs a record containing an array and scalar count,
extracts one element/reduction, and uses the result in arithmetic, a branch,
and serialization. No change is required in `../demo-file-processing`.

## String-list concatenation for multi-pattern discovery — open

The audio picker needs to combine the deterministic results of
`fs_walk(..., pattern: "*.mp3")` and `fs_walk(..., pattern: "*.ogg")`.
`concat` is array-only, while `list_append`/`list_concat` remain documented
future capabilities; attempting the array operation on string lists terminates
the MLPL worker with `expected an array value, got a string`.

This repository currently injects one bounded, sorted MP3/Ogg catalog from the
confined Rust host. Required upstream contract: `list_concat(xs, ys) ->
string-list`, preserving order and accepting empty lists, plus tests for two
non-empty `fs_walk` results. This is a language collection primitive, not an
audio-specific builtin. Once shipped, discovery can move entirely into the
MLPL picker without changing decoder or renderer APIs.
