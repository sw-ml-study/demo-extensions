# sw-MLPL Blockers for Interactive Native 3D

Status date: 2026-08-13

The native wgpu/winit renderer and MLPL-generated cube scene work today. The
remaining blocker is not drawing a cube; it is letting MLPL own a persistent,
interactive native application through the public third-party extension API.
This repository does not modify `../sw-mlpl`.

## Required host primitives

### Typed persistent native handles

sw-MLPL must transport extension-scoped, type-tagged generational handles as
language values. A viewer returned by `native3d:open(...)` must remain valid
across later MLPL evaluations and reject stale, forged, wrong-type, and
cross-extension handles. Deactivation must finalize viewers deterministically.

Downstream acceptance: an MLPL test creates, queries, closes, and then fails to
reuse a viewer; negative tests cover every invalid-handle class and repeated
close.

### Bounded event polling and host event-loop ownership

The extension needs a non-callback primitive such as
`native3d:poll_events(viewer, limit)` that returns ordered key, pointer, resize,
and close records. The host must specify macOS main-thread ownership, Linux
event-loop behavior, queue bounds, backpressure, cancellation, REPL lifetime,
and shutdown. Polling is preferred for the PoC because it avoids language
reentrancy from native callbacks.

Downstream acceptance: MLPL opens a viewer, polls bounded batches, handles
resize and close, and exits cleanly under repeated open/poll/close cycles.
Overflow and provider deactivation have deterministic results.

### Bulk arrays through the public C-provider boundary

The host C adapter must accept dense numeric arrays with dtype, rank, shape,
strides, ownership, and call lifetime. The PoC needs `[N,3]` positions and
`[M,2]` edges plus parallel color, thickness, and ID arrays. Contiguous,
read-only, call-scoped borrowing is sufficient initially; unsupported layouts
must fail with useful diagnostics.

Downstream acceptance: one MLPL call updates a complete line scene in bulk.
Tests cover wrong dtype/rank/shape, invalid indices, overflow, non-contiguous
storage, expired storage, and explicit copy-versus-borrow evidence.

### Generic viewer calls exposed to MLPL

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

### Interpreted and compiled parity

The same MLPL application must resolve the provider and behave consistently in
the REPL, interpreted scripts, and eventually compiled binaries. The compiler
needs an extension registration/startup hook and artifact packaging or static
provider linkage without changing MLPL source.

Downstream acceptance: the same cube source runs interpreted and compiled,
with equivalent events, scene updates, diagnostics, and close behavior.

## What works while blocked

`just cube-3d` already proves the visual portion on macOS: sw-MLPL generates
the deterministic generic scene, a temporary JSON file crosses the current
boundary, and the standalone Rust smoke viewer displays the rotating cube in a
native window. Headless Rust tests prove transforms, clipping, projection,
line expansion, style, bounds, and a stable image fingerprint. Native
mlplunit tests prove the MLPL cube arrays and controls.

The JSON bridge copies the complete scene and cannot return live events. The
standalone smoke harness advances rotation from the MLPL-provided speed and
supports only Escape/native-close lifecycle behavior. It does not prove the
extension API or the MLPL-owned interactive loop.

## Upstream handoff order

1. Carry typed native handles through the host value model and C adapter.
2. Carry bounded dense arrays through the same adapter.
3. Define main-thread event-loop ownership and add bounded polling.
4. Prove the generic viewer functions through interpreted MLPL.
5. Add provider startup, linkage/package, and call parity to compiled output.

Each upstream delivery should be followed by the named downstream acceptance
test here. Until then, the PoC remains visually runnable but partially blocked
for end-to-end interactive MLPL ownership.
