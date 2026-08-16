# Extension Blockers and Host Requirements

Status date: 2026-08-13

This document is the handoff contract between `demo-extensions` and
`../sw-mlpl`. It distinguishes capabilities already proven from work that is
still blocked on the host. A blocker is closed only by the named downstream
acceptance evidence; an upstream implementation or synthetic fixture alone is
not sufficient.

For the focused native-3D handoff, see
[`sw-mlpl-blockers.md`](sw-mlpl-blockers.md).

## Proven baseline

The following are no longer blockers:

- ABI V1 layout, bounded descriptor validation, owned metadata, scalar values,
  errors, panic containment, arrays, and opaque handles are implemented here.
- Dynamic and static providers use the same downstream registration path.
- The independently built hello shared library is loaded, retained, invoked,
  deactivated, and package-resolved by downstream Rust tests.
- `sw-mlpl` accepts this repository's byte-compatible static C descriptor via
  `register_c_extension` and dispatches `_hello:answer()` and `_hello:fail()`
  through its interpreter. Evidence:
  `tests/upstream-host/tests/c_provider.rs`.
- The installed host's built-in static provider is callable from MLPL as
  `hello:answer()`. Evidence: `tests/test_upstream_static_registry.mlpl`.

The shipped host adapter is static and scalar-only. It does not close any of
the blockers below by implication.

## Blocker summary

| ID | Capability | Owner | Current limitation | Dependency |
|---|---|---|---|---|
| B1 | `use`/facade surface | sw-MLPL | Only colon-qualified native names are callable | modules/namespaces and facade saga |
| B2 | Compiler parity | sw-MLPL | Extension calls work only in interpreter/REPL paths | compiler I/O parity, then extension startup hook |
| B3 | Dynamic host loading | sw-MLPL | Host adapter accepts process-resident static descriptors only | loader, manifest, trust, and lifecycle policy |
| B4 | Dense arrays at host boundary | closed | Real provider round-trip proven through adjacent interpreter | `data_boundary.rs` |
| B5 | Native handles at host boundary | closed | Persistent, closed, stale, foreign, and malformed cases proven through adjacent interpreter | `data_boundary.rs` |
| B6 | Event loop and callbacks | sw-MLPL + extension | No host policy for native windows or event delivery | B5 plus main-thread/reentrancy policy |
| B7 | Package discovery and trust | sw-MLPL | Downstream manifests are not a host search/load contract | B3 and deployment policy |
| B8 | C-provider help metadata | sw-MLPL | Adapter registers empty signature metadata | metadata parsing and catalog bridge |

## B1: `use` and facade publication

Requirement:

- `use hello` registers the native provider before evaluating
  `extensions/hello/module.mlpl`.
- The public facade uses dotted names such as `hello.answer()` while private
  `_hello:*` implementation names remain inaccessible to ordinary modules.
- REPL and interpreted scripts apply identical resolution and diagnostics.
- Missing package, duplicate namespace, facade parse failure, and public/private
  collisions fail deterministically without partial publication.

Acceptance:

- A native mlplunit test executes `use hello` and calls a public facade
  function without injecting the native result as a test argument.
- Namespace-policy checks prove user code does not call `_hello:*` directly.

Current workaround: call a registered provider with colon spelling, for
example `_hello:answer()`. This is integration evidence, not the intended user
surface.

## B2: compiled-program parity

Requirement:

- Generated Rust has a link-time static-provider registration hook that calls
  the same host registry used by the interpreter before user code executes.
- The same MLPL source and namespace spelling behave identically in scripts,
  the REPL, and compiled binaries.
- Typed values, extension failures, and contained panics preserve their
  interpreter semantics and exit behavior.
- Duplicate or absent providers produce stable build/startup diagnostics.

Acceptance:

- Compile and run a program that invokes this repository's hello provider and
  returns `42`.
- Compile/run negative fixtures for extension failure, contained panic,
  missing provider, and duplicate registration.

Current workaround: none. Downstream Rust static-provider tests do not prove
the MLPL compiler startup path.

## B3: dynamic loading and lifecycle

Requirement:

- The host loads a platform artifact, resolves only
  `sw_mlpl_extension_v1`, validates ABI version and descriptor bounds, then
  registers through the same safe registry used by static providers.
- The library remains resident while functions, calls, callbacks, or handles
  can reference its code or data.
- Load failure is atomic. Unload/reload requires a demonstrated quiescence
  protocol; otherwise unload remains unsupported.
- Panics, malformed results, and missing symbols cannot unwind into or corrupt
  the host.

Acceptance:

- An MLPL script loads this repository's independently built hello artifact
  and calls success/failure/panic cases.
- Negative fixtures cover missing symbol, wrong ABI/layout, malformed
  descriptor, duplicate namespace, active-call/handle unload, and reload.

Current workaround: this repository proves dynamic loading only in its Rust
harness; `sw-mlpl` proves only static C-provider registration.

## B4: dense arrays

Requirement:

- The host maps arrays to ABI dtype, rank, shape, byte strides, mutability,
  ownership, and data address with checked size/offset arithmetic.
- Storage is rooted and immovable for the complete native call.
- The initial contract may accept contiguous, read-only arrays only; unsupported
  layouts must fail with actionable dtype/rank/shape/stride diagnostics.
- Copy versus borrow behavior is explicit and measured. No zero-copy claim is
  permitted without evidence from actual host storage.

Acceptance:

- MLPL sends one f32 `[N,3]` array to `_hello:sum_positions` in one native call
  and receives the expected scalar.
- Fixtures reject wrong dtype/rank/shape, non-contiguous strides, overflow,
  misalignment, excessive size, mutation requests, and expired call storage.

Closed for interpreted static providers: `_boundary:echo_array` round-trips a
dense `[2,3]` array through the real C adapter and rejects a non-array value.
Copy/borrow performance and unsupported-layout coverage remain future depth,
not blockers for the current PoC.

## B5: native handles

Requirement:

- The MLPL value model carries extension identity, type identity, slot/object
  ID, and generation; no resource pointer crosses the language boundary.
- Lookup validates extension, declared type, stored type, slot, generation, and
  provider activity before resource access.
- Removal invalidates before finalization. Generation exhaustion retires slots
  instead of wrapping. Deactivation finalizes deterministically.
- Values cannot be forged through ordinary numeric construction or reused
  across extensions.

Acceptance:

- End-to-end tests cover create/use/drop, stale generation, wrong type,
  cross-extension use, exhaustion, duplicate drop, provider deactivation, and
  deterministic finalization order.

Closed for interpreted static providers: a real MLPL variable retains the
provider handle across calls, while closed, stale, cross-extension, and
non-handle values fail cleanly in `data_boundary.rs`.

## B6: event loop, callbacks, and persistent resources

Requirement:

- Define ownership of the macOS/Linux main thread and compatibility with
  repeated REPL evaluation.
- Event queues are bounded and specify ordering, backpressure, cancellation,
  reentrancy, callback threading, and behavior after deactivation.
- Host calls never hold evaluator locks while extension callbacks re-enter the
  language unless an explicit reentrancy protocol permits it.
- Window/resource handles follow B5 lifecycle rules.
- The extension exposes generic `open`, bounded `poll_events`, bulk
  `set_lines`, explicit `render`/`present`, time/size queries, and `close`
  primitives. MLPL owns the application loop and maps events to domain state;
  no cube-specific controls are implemented in Rust.

Acceptance:

- A headless lifecycle harness and a real-window smoke test cover repeated
  open/update/poll/close, bounded events, callback errors, and shutdown.

Current workaround: `just cube-3d` opens a real opt-in wgpu/winit window from
an MLPL-generated scene and proves native rendering and close behavior. Its
standalone smoke harness advances rotation from the MLPL-provided speed, but it
cannot return a handle or events to MLPL. It is visual evidence only, not proof
of open/update/poll/close through the public extension API.

Downstream progress: `_native3d` now proves the generic headless create,
bulk-update, state/size, render-state, close, stale-handle, and deactivation
slice through the real interpreter. Only window/event-loop delivery remains
blocked for interpreted interaction.

## B7: package discovery, deployment, and trust

Requirement:

- The host consumes a versioned manifest with public name, private namespace,
  ABI version, MLPL facade, exact platform artifacts, and integrity metadata.
- Search paths, precedence, canonical path confinement, symlink behavior,
  supported triples, duplicate packages, and offline deployment are
  deterministic.
- Native extensions are explicitly treated as trusted process code; signature
  or allow-list policy is documented before automatic loading.

Acceptance:

- The downstream manifest fixtures run through the host resolver on macOS and
  Linux, including traversal, mismatch, duplicate, missing artifact, integrity,
  and unsupported-platform failures.

Current workaround: the downstream package resolver is proven, but `sw-mlpl`
does not consume it.

## B8: C-provider signatures and help

Requirement:

- The C adapter parses the copied TOML metadata document and validates exact
  function-name/arity agreement before registration.
- Function documentation, ordered arguments, defaults, return types, and native
  types enter the same help catalog used by built-ins and safe static providers.
- Malformed, duplicate, incompatible, or drifting metadata rejects the complete
  provider atomically.

Acceptance:

- `:describe _hello:answer` and facade help render the canonical downstream
  signature/documentation.
- Existing malformed and stable-help fixtures are exercised through the host
  adapter, not only through the downstream registry.

Current workaround: downstream help is deterministic, while the host C adapter
currently registers empty signature metadata.

## Completion rules

For every blocker:

1. Implement upstream without changing this repository unless the public
   contract genuinely requires a downstream adaptation.
2. Add a downstream test that uses the real public host API and this provider.
3. Run `mlplunit` for MLPL files and `cargo test` for Rust code, followed by
   `just check` here and the upstream pre-commit gate.
4. Update this document and the relevant acceptance report with a named test,
   exact limitation, execution modes, and copy/ownership claims.
5. Commit and push both repositories independently; never hide an unresolved
   mode behind a mock or synthetic provider.
