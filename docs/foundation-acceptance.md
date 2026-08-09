# Extension Foundation Acceptance

Date: 2026-08-08

Saga: `extension-foundation`

Scope: downstream ABI, loader, package resolver, hello extension, and MLPL
facade. No files in `../sw-mlpl` were changed.

## Result

The downstream foundation is accepted with an explicit upstream integration
blocker. An independently built hello shared library is loaded through the V1
C ABI, validated, registered under a private namespace, invoked, deactivated,
and packaged deterministically. Its public MLPL facade is executable and tested
separately. sw-MLPL cannot yet connect that facade to the native registry.

## Evidence

| Contract | Evidence | Result |
|---|---|---|
| C layout, tags, ABI/size negotiation | `contract.rs`: 6 tests | Proven |
| Malformed metadata fails closed | null/count/UTF-8/reserved/duplicate tests | Proven within documented readable-pointer precondition |
| Host owns registered metadata | `validation_copies_extension_owned_metadata` | Proven |
| Panic does not unwind across C | ABI containment test and `_hello.panic` dynamic test | Proven; the Rust panic hook still prints |
| Independent shared-library loading | `hello_registration.rs`: 4 tests | Proven on the current macOS host; Linux artifact naming is platform-derived and compile support is declared, not runtime-tested here |
| Namespaced result/error invocation | `_hello.answer`, `_hello.fail`, `_hello.panic` | Proven for zero-argument `i64` V1 slice |
| Deactivation and library lifetime | `registry_retains_library_until_deactivation` | Proven for registry ownership; true `dlclose`/hot reload remains deferred |
| Deterministic package resolution | `manifest_resolution.rs`: 3 tests | Proven for exact targets, path confinement, missing files, duplicates, ABI/platform mismatch, and stable diagnostics |
| Public/private facade separation | mlplunit: 2 tests plus `check-namespaces` | Proven for the current explicit-value facade |
| Complete repository gate | `just check` | Passed: layout, ignore/tracking/namespace audits, compile, clippy, all Rust tests, mlplunit, and whitespace |

The intentional `_hello.panic` case prints a panic-hook message during Rust
tests. The process continues, the foreign call returns the V1 panic status, and
the host receives `ExtensionPanicked`; no unwind crosses `extern "C"`.

## Execution-mode classification

| Mode | Classification | Evidence and limitation |
|---|---|---|
| Downstream Rust host harness | Proven | Loads the independently built `.dylib`/`.so` shape and invokes all hello functions through `libloading`. |
| sw-MLPL REPL | Blocked upstream | No public evaluator/REPL hook registers a validated external descriptor or resolves `use hello`. |
| Interpreted `.mlpl` script | Limited, native invocation blocked | mlplunit proves `extensions/hello/module.mlpl` parses, loads, and executes as ordinary MLPL. It receives the native value explicitly because the interpreter has no extension registry/import bridge. |
| Compiled MLPL program | Blocked upstream | No compiler/package hook embeds a static provider or deploys and registers a dynamic package while preserving the same MLPL namespace. |

The repository therefore does not claim end-to-end `.mlpl` native invocation.
That claim requires the host contracts in `upstream-contract.md` under a
separately authorized `../sw-mlpl` task.

## Static-provider parity decision

Static-provider parity is required and becomes the first step of the next
downstream saga. The registry must validate and register a provider through one
common path whether its descriptor comes from a retained `libloading::Library`
or a statically linked entry function. This avoids making compiled MLPL source
or extension APIs differ from REPL/script APIs.

The next implementation should introduce a provider guard with dynamic and
static variants, factor descriptor registration out of `Registry::load`, and
run the same hello behavior tests against both providers. It must not pretend
that this local parity supplies the missing sw-MLPL compiler hook.

## Deferred and blocked work

- General arguments/results, rich signatures, dense arrays, and typed native
  handles belong to the next SDK saga.
- True unload/hot reload remains deferred until active calls, handles,
  callbacks, threads, and host services can prove quiescence.
- Linux runtime smoke evidence needs a Linux execution environment; macOS is
  the only runtime exercised by this acceptance run.
- REPL, interpreted-native, and compiled integration require upstream work and
  are the current blocker for user-visible `use hello` behavior.

## AgentRail audit note

`agentrail audit` reports every implementation commit matched to a saga step
and no orphan steps. It also reports bootstrap and post-completion metadata
commits as orphan commits because this repository deliberately commits
AgentRail's completion metadata after `agentrail complete`, as required by its
publication policy. Pre-existing user work in `README.md`, `COPYRIGHT`, and
`LICENSE` remains uncommitted and untouched.
