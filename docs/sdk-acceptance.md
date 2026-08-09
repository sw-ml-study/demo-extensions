# Extension SDK Acceptance

Date: 2026-08-09

Saga: `extension-sdk-arrays-handles`

## Result

The downstream ABI/SDK is accepted for provider-neutral Rust hosting. Dynamic
and static providers share one loader registration path, hello is authored
with safe handlers and generated ABI boilerplate, all scalar and rich metadata
contracts fail closed, a dense `[N,3]` array crosses in one call, and native
resources cross only as opaque generational capabilities.

## Evidence

| Contract | Evidence | Result |
|---|---|---|
| Dynamic/static parity and lifetime | `hello_registration.rs` | Proven for answer, failure, panic, bulk array, deactivation, and namespace checks |
| Safe authoring surface | `check-sdk-authoring`; hello source | Proven: no handwritten `unsafe` or direct ABI dependency in hello source |
| Scalar ownership and malformed values | `scalars.rs` | Proven |
| Signatures, defaults, types, stable help | `metadata.rs` plus hello registry help | Proven |
| Dense numeric arrays | `arrays.rs`; `_hello.sum_positions` | Proven with one defensive foreign-to-owned copy; no end-to-end zero-copy claim |
| Typed native handles | `handles.rs` | Proven for identity, stale/type/extension rejection, exhaustion, and deterministic finalization |
| Repository gate | `just check` | Passed: policies, formatting, compile, clippy, Rust tests, mlplunit, and whitespace |

## sw-MLPL execution modes

The installed release binary reports build commit `91d5216a`. A temporary
interpreted script containing `hello:answer()` returned `42`, and the REPL
command `:describe hello:answer` returned the native signature and canonical
documentation. This proves the shipped upstream static scalar registry in
script and REPL modes.

The host's built-in provider uses its safe `ExtValue` descriptor. The shipped
`mlpl-extension-cabi` adapter now additionally publishes a byte-for-byte V1 C
layout and accepts this repository's `static_entry()` descriptor pointer.
`upstream_c_provider.rs` links those public upstream crates read-only, registers
the real downstream provider, and proves `_hello:answer()` returns `42` while
`_hello:fail()` becomes an MLPL error Result.

| Mode | Classification |
|---|---|
| Downstream dynamic and static Rust providers | Proven through the same validated registry path |
| sw-MLPL interpreted/REPL static scalar registry | Proven by `test_upstream_static_registry.mlpl` and local REPL description |
| This hello C descriptor inside sw-MLPL | Proven for statically linked scalar success and failure through `upstream_c_provider.rs` |
| `use hello` and dotted facade | Blocked on the queued facade/module saga |
| Compiled extension invocation | Blocked on compiler parity |
| Packaged dynamic loading | Blocked on host dynamic loading, trust, and manifest integration |
| Host arrays and native handles | Blocked on upstream value/lifetime contracts |

No file in `../sw-mlpl` was changed during this saga.
