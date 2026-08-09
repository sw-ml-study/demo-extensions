# Development and Testing

Run repository workflows through `just`:

```sh
just tests
just rust-tests
just check
```

`just check` is the mandatory pre-commit gate. It checks shell syntax, the
repository layout, `.gitignore`, tracked source/docs/configuration, Rust
formatting, compilation, clippy, workspace tests, native mlplunit tests, and
whitespace errors.

## Tool selection

The scripts never install or replace developer tools. `scripts/select-mlpl`
uses the absolute `$MLPL` override when set, otherwise the release and debug
builds in `../sw-mlpl`. `scripts/select-mlplunit` uses absolute `$MLPLUNIT`,
then PATH, then `/Users/mike/github/softwarewrighter/mlplunit/bin/mlplunit` as
resolved through the adjacent-checkout convention.

Overrides must be absolute paths so a test cannot silently select a different
binary after changing its working directory.

## TDD contract

Rust changes begin with the smallest failing unit or integration test and use
scoped `cargo test` while iterating. MLPL tests live under `tests/`, match
`test_*.mlpl`, register behavior with `@test`, use shared `u:assert_*` helpers,
and finish with one `u:run_registered_tests()` call.

The initial crates contain documentation-only facades. ABI types, loader
behavior, SDK conversions, and hello registration are intentionally deferred
to their AgentRail steps so each begins from an observable failing test.
