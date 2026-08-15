# sw-MLPL Data-Boundary Acceptance

Date: 2026-08-15

The repository's real public SDK descriptor now passes dense arrays, typed
native handles, and nested records through the actual adjacent sw-MLPL
interpreter. The acceptance harness links the provider statically and calls
the public `register_c_extension` adapter; it does not replace either side
with a test-only value converter.

## Evidence

| Contract | Executable evidence | Result |
|---|---|---|
| Dense array input and result | `data_boundary.rs` calls `_boundary:echo_array([[1,2,3],[4,5,6]])` | Shape `[2,3]` and all values survive both directions |
| Persistent native handles | A handle is stored in an MLPL variable and consumed by a later native call | Proven |
| Invalid capabilities | Closed, stale, and foreign handles plus a numeric non-handle are rejected as MLPL error results | Proven |
| Structured results | `_boundary:event_batch()` returns nested records accessed as `events.e0.x`, `events.e1.x`, and `events.count` | Proven |
| Malformed SDK values | ABI/SDK tests reject invalid pointers, counts, names, duplicate fields, and excessive nesting | Proven below the real interpreter boundary |

Run the focused host proof with:

```sh
CARGO_TARGET_DIR=target/upstream-host cargo test \
  --manifest-path tests/upstream-host/Cargo.toml --test data_boundary
```

The repository-wide `just check` also runs this isolated adjacent-host test
and the native mlplunit suites.

## Scope and limitations

The `_boundary` provider is intentionally generic and statically linked into
the isolated host harness. The installed `mlpl-repl` does not bootstrap this
repository's provider, so a native mlplunit suite cannot name `_boundary:*`
without a future package/startup mechanism. Existing mlplunit suites continue
to prove MLPL-owned scene and control behavior.

This closes the interpreted static-provider data-boundary gate. It does not
prove dynamic host loading, `use` facades, compiler startup/linkage, or
zero-copy arrays. The event records are deterministic synthetic data; real
winit event delivery and host event-loop ownership remain the blocker for an
end-to-end interactive MLPL window. No file in `../sw-mlpl` was modified.
