# Upstream sw-MLPL Contract

This downstream repository must not modify `../sw-mlpl`. It records the host
capabilities required to make the examples real. Each item needs an upstream
implementation or an already-supported public equivalent before downstream
acceptance can claim end-to-end MLPL integration.

## Required foundation contracts

- A public registry API for namespaced native functions and native value types.
- A versioned value/error boundary independent of evaluator implementation
  details and Rust ABI stability.
- Module/package resolution capable of loading an MLPL facade after native
  registration.
- Equivalent registration hooks for REPL, interpreted scripts, and compiled
  programs; compiled programs may use a static provider initially.
- Help/signature metadata and actionable argument/type/shape diagnostics.

## Required array and resource contracts

- Dense numeric array borrowing with dtype, rank, shape, strides, mutability,
  ownership, and call-lifetime explicitly represented.
- Rooting or ownership rules that prevent array storage from moving or being
  freed during a native call.
- A native-handle value carrying extension identity, type identity, and a
  generational object ID, plus deterministic finalization/deactivation rules.
- Panic/error containment so an extension failure cannot unwind through the
  host or corrupt evaluator state.

## Required interaction and deployment contracts

- A macOS/Linux host event-loop policy compatible with winit's main-thread
  requirements and repeated REPL evaluation.
- Bounded event delivery with documented reentrancy and callback policy.
- Compiler/package hooks for dynamic artifacts and, later, statically linked
  providers without changing MLPL source.
- Deterministic extension search paths, platform triples, manifest validation,
  and a trust/integrity policy.

### Native application primitive surface

The downstream renderer must surface generic primitives through the public
extension boundary rather than implement application behavior in Rust:

- create a native window/viewer and return a typed generational handle;
- poll a bounded batch of key, pointer, resize, and close events as MLPL data;
- update positions, edges, and parallel style/ID arrays in bulk;
- render/present using explicit time or rotation state supplied by MLPL;
- query monotonic time and current drawable size;
- close/deactivate deterministically and reject stale handles.

MLPL owns the loop and maps events to cube dimensions, rotation speed, color,
and thickness. This same source should remain compilable when compiler parity
lands. The native implementation may translate winit event variants and wgpu
resources, but must not encode cube controls or other application semantics.

## Capability reporting

Every foundation acceptance report classifies each item as supported, proven
by a named test; supported with a limitation; or blocked upstream. A local mock
registry may test the ABI and SDK, but it is never evidence that REPL, script,
or compiled sw-MLPL integration already works.

### Confined file metadata

Native file-selection applications require modification timestamps from the
same sw-MLPL sandbox that governs `fs_walk`, `file_size`, and bounded byte
reads. Interpreter commit `0f4d0e32` now exposes the required fallible,
platform-neutral Unix-millis value through `file_metadata` without allowing
paths outside the configured root. Formatting, timezone choice, sorting, and
UI presentation remain MLPL application semantics. Compiled lowering and a
configured-root parked-main helper remain upstream gaps.
`demo-file-processing` should validate and teach the shipped primitive; it is
a consumer, not the primary API owner. See
[sw-MLPL blockers](sw-mlpl-blockers.md#confined-filesystem-modification-times--open)
for exact failure and acceptance behavior.

## Current integration status

The downstream `hello_registration` Rust integration test now proves dynamic
loading, V1 validation, namespaced invocation, typed result/error copying,
panic containment, library retention, and deactivation. This is not a mock of
the binary boundary: it loads the independently built hello shared library.

The installed `sw-mlpl` release binary supports actual MLPL invocation with
colon-qualified names. `test_upstream_static_registry.mlpl` proves
`hello:answer()` returns `42`, and a local REPL probe proves
`:describe hello:answer` exposes its signature and documentation. The public C
adapter now accepts this repository's byte-compatible static descriptor;
`tests/upstream-host/tests/c_provider.rs` proves `_hello:answer()` and
`_hello:fail()` through the upstream interpreter. The native3d integration now
also proves interpreted static-provider arrays, typed persistent handles,
parked-main event-loop ownership, and bounded Port delivery. `use` facade
resolution, compiled-provider parity, and packaged dynamic loading remain
upstream work. No upstream files are changed by this repository.

### Execution modes as of foundation acceptance

- REPL/interpreted scalar invocation: proven through the static registry's
  colon spelling.
- Downstream C descriptor registration: proven for a statically linked scalar
  provider through the public upstream adapter.
- Interpreted native3d arrays, handles, and bounded interaction: proven through
  the static provider and worker/Port application path.
- `use hello` facade invocation: blocked pending the upstream facade saga.
- Compiled native invocation: blocked; no static-provider or packaged dynamic
  provider hook exists in the compiler/runtime contract.
- Downstream Rust harness: proven by `hello_registration.rs`; it is evidence
  for the proposed boundary, not evidence that sw-MLPL already implements it.

### Outbound records and packed bytes -- resolved

The public static C-descriptor hook now runs the real `_http:get(string)`
provider from `demos/http-client/get.mlpl`; its mandatory acceptance uses a
loopback server. sw-MLPL revision `635e085b` now recursively marshals MLPL
records and packed bytes through the existing ABI V1 record tag. Upstream tests
cover interpreter dispatch, a real C-provider invocation, borrowed-view
ownership, a 1,024-field per-level cap, and a 64-level nesting cap. No ABI
layout change or application-specific hook was needed. Package `use` resolution
and compiled-provider parity remain separate blockers.

The same bridge unblocks interpreted calls to `_http.request(record)`,
`_sqlite.open(config)`, `execute(..., params)`, and `query(..., params)`.
End-to-end downstream composition is separate acceptance work; the providers
require no alternate ABI or application-specific host hook.

The MLPL web framework and TodoMVC use those exact future-ready records for
`_web.listen`/`respond` and `_sqlite.open`/`execute`/`query`. Their model,
router, middleware, authorization, encodings, sessions, controllers, views, and
parameterized persistence plans run under native mlplunit. The upstream value
blocker is closed, and `scripts/check-todomvc-live` now proves browser-to-server
composition, persisted restart state, and explicit reset downstream.

### Extension handles through user functions

Live TodoMVC regression against sw-MLPL `f569defa` found that a native
extension handle can be stored in a top-level MLPL binding and passed directly
to another extension call, but cannot bind to an ordinary `u:` function
parameter. The evaluator rejects that parameter kind before the function body.
This does not block TodoMVC: `server.mlpl` keeps listener/connection operations
in its top-level application loop while all routes, state, SQL plans, and HTML
remain in MLPL functions. General handle-accepting library helpers require the
separately authorized upstream follow-up recorded in AgentRail step 012.

The same checkout's freshly rebuilt executables report older commit IDs from
`mlpl-repl --version` (`01fd675a` debug, `2b11c6b9` release) rather than
`f569defa`. Source-linked and live regressions pass, so this is build-metadata
freshness evidence rather than a runtime blocker.

See `foundation-acceptance.md` for the complete evidence matrix and limitations.
See `extensions-blockers.md` for the actionable requirements and acceptance
criteria for every remaining host capability.
