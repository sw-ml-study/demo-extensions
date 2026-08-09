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

As of reported `sw-mlpl` binary `c3452aa1`, a static registry supports actual
MLPL invocation with colon-qualified names. The next acceptance step must prove
that path against hello before this repository marks it supported. `use` facade
resolution, compiler parity, packaged dynamic loading, arrays at the host
boundary, and native-handle values remain upstream work. No upstream files are
changed by this repository.

### Execution modes as of foundation acceptance

- REPL/interpreted scalar invocation: reported available through the static
  registry's colon spelling; downstream acceptance is pending.
- `use hello` facade invocation: blocked pending the upstream facade saga.
- Compiled native invocation: blocked; no static-provider or packaged dynamic
  provider hook exists in the compiler/runtime contract.
- Downstream Rust harness: proven by `hello_registration.rs`; it is evidence
  for the proposed boundary, not evidence that sw-MLPL already implements it.

See `foundation-acceptance.md` for the complete evidence matrix and limitations.
