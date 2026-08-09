# Extension Foundation

Establish the repository gate and prove the smallest public extension boundary
before graphics enters the dependency graph. Work is downstream-only: missing
sw-MLPL host capabilities are recorded in `docs/upstream-contract.md` and are
not silently implemented in the adjacent repository.

1. `repository-tdd-scaffold` — Add the Rust workspace skeleton, MLPL source and test layout, root mlplunit configuration, tool-selection scripts, thin just recipes, and structural tests for the intended package layout.
2. `abi-v1-contract` — Specify minimal C-safe descriptor/value/error types and write Rust tests for layout, version negotiation, malformed descriptors, ownership, and panic containment before implementing them.
3. `hello-extension-registration` — Build an independently compiled hello cdylib and a safe test registry that loads, validates, registers, invokes, reports errors, retains its library lifetime, and deactivates it.
4. `manifest-and-module-facade` — Add deterministic manifest/platform resolution and an MLPL module facade, with traversal, mismatch, missing-artifact, duplicate-name, and diagnostic tests.
5. `foundation-acceptance-report` — Run Rust and MLPL gates, document REPL/script/compiled capability evidence and upstream gaps, decide static-provider parity, reconcile the next saga, and stop.

Acceptance: the headless hello path is independently built and contract-tested;
the user-facing namespace hides FFI; malformed or incompatible extensions fail
closed; and every unavailable upstream integration is documented rather than
mocked as complete.
