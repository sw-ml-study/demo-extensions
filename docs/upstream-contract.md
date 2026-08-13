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
`_hello:fail()` through the upstream interpreter. `use` facade resolution, compiler parity, packaged
dynamic loading, arrays at the host boundary, and native-handle values remain
upstream work. No upstream files are changed by this repository.

### Execution modes as of foundation acceptance

- REPL/interpreted scalar invocation: proven through the static registry's
  colon spelling.
- Downstream C descriptor registration: proven for a statically linked scalar
  provider through the public upstream adapter.
- `use hello` facade invocation: blocked pending the upstream facade saga.
- Compiled native invocation: blocked; no static-provider or packaged dynamic
  provider hook exists in the compiler/runtime contract.
- Downstream Rust harness: proven by `hello_registration.rs`; it is evidence
  for the proposed boundary, not evidence that sw-MLPL already implements it.

See `foundation-acceptance.md` for the complete evidence matrix and limitations.
See `extensions-blockers.md` for the actionable requirements and acceptance
criteria for every remaining host capability.
