# C Provider Host Acceptance

Date: 2026-08-09

`sw-mlpl` now publishes `mlpl-extension-cabi`, whose V1 model is documented as
byte-for-byte identical to this repository's C ABI. A standalone downstream
acceptance crate links that public adapter, parser, and evaluator as read-only
path dependencies. It is excluded from the main workspace so formatting and
lint discovery cannot cross into upstream source; its check script directs all
compilation output into this repo's ignored `target/upstream-host/` directory.

The test passes `mlpl_extension_hello::static_entry()` directly to
`register_c_extension`, then evaluates `_hello:answer()` and
`is_ok(_hello:fail())`. The private `_hello` namespace distinguishes this
descriptor from sw-MLPL's built-in safe `hello` provider. Success returns `42`;
the extension failure crosses the C boundary as an MLPL error Result.

This closes the primary static C-descriptor registration blocker. It does not
claim `use hello`, dotted facade publication, compiled-program registration,
dynamic library loading, host array borrowing, or native-handle transport.
Those remain the queued upstream capabilities. The adapter currently rejects
array and handle tags and registers static descriptors only.
