# Confined SQLite Extension

## Scope and ownership

`extensions/sqlite` provides generic local relational persistence through the
same public V1 C ABI, SDK, loader, manifest, and package shape available to an
external extension author. Rust owns SQLite calls, path confinement, prepared
statement lifetimes, connection and transaction state, resource limits, and
cleanup. MLPL owns schemas, SQL, parameter values, authorization, validation,
routes, controllers, and application meaning.

V1 deliberately exposes only typed connection handles. Prepared statements
live for one `execute` or `query` call, so they cannot outlive their borrowed
connection. Queries materialize a bounded result, so no cursor resource remains
after the call. Explicit transaction state is held by the connection and is
changed only by `begin`, `commit`, and `rollback`. This avoids statement,
cursor, and transaction handles whose lifetimes would add complexity without
benefit at the teaching-scale limits.

## Open and confinement

`_sqlite.open(config)` accepts exactly:

```text
{
  root: "/absolute/existing/directory",
  path: "data/runs.sqlite",
  busy_timeout_ms: 1000,
  max_rows: 1000,
  max_result_bytes: 1048576
}
```

The root is canonicalized and must already be a directory. The database path
must be relative and contain only normal components—no absolute path, `.` or
`..`. Its parent must already exist and canonicalize beneath the root. An
existing database is canonicalized again, which rejects a symlink to a target
outside the root. SQLite is opened read/write with create enabled, foreign keys
enabled, trusted schemas disabled, and a caller-bounded busy timeout from 0 to
60 seconds.

This is process-local path confinement, not a replacement for an operating
system sandbox. A hostile process that can exchange filesystem objects during
the check/open interval is outside V1's threat model. Deployment should grant a
dedicated non-hostile root with suitable OS permissions.

At most 32 connections exist on one thread. Connections cross MLPL as typed
generational handles. Foreign, wrong-type, closed, and stale handles fail before
SQLite access. `close` explicitly rolls back an active transaction before
removing the resource; dropping a process/thread also closes held connections.

## Parameterized execution

`execute(connection, sql, params)` and `query(connection, sql, params)` prepare
exactly one SQL statement. SQL is limited to 64 KiB. Parameters use an indexed
record so order and mixed types survive ABI V1:

```text
{count: 2, item_0: "run-1", item_1: 0.875}
```

The provider requires the record count to match SQLite's placeholder count.
Supported values are nil, bool, i64, finite f64, UTF-8 string, and bytes. Use
`?1`, `?2`, and so on; do not concatenate request or authentication data into
SQL. The API structurally separates SQL and parameters, although no SQL parser
can determine whether an application embedded an unsafe literal itself.

`execute` returns `{rows_affected, last_insert_rowid}`. Transaction-control SQL
is rejected by execute/query; applications use the explicit lifecycle calls.
Nested begin and commit/rollback without an active transaction fail closed.

## Bounded query result

`query` returns duplicate column names and mixed row types without treating
names as record keys:

```text
{
  count: 1,
  columns: {count: 2, item_0: "name", item_1: "score"},
  row_0: {count: 2, item_0: "run-1", item_1: 0.875}
}
```

The open configuration fixes `max_rows` in 1..10,000 and
`max_result_bytes` in 1..16 MiB. The byte budget counts column-name UTF-8 and
each returned scalar/text/blob payload; fixed record-key/container overhead is
not included. Queries are also limited to 256 columns. Crossing either bound
fails the whole call—no partial result is returned.

## Acceptance and current host limitation

`extensions/sqlite/tests/sqlite_contract.rs` covers parameter binding, typed
results, commit/rollback, rollback-on-close, stale handles, row/byte limits,
path traversal, and symlink escape. `provider_contract.rs` proves matching
dynamic and static packages. `tests/test_sqlite_module.mlpl` covers the public
facade's parameter records with native mlplunit.

The current sw-MLPL static-provider outbound adapter still cannot marshal MLPL
records or packed bytes. Therefore the native provider is proven at the ABI and
loader layers, but an interpreted program cannot yet call `sqlite:open` through
the package facade. This is the same upstream record/bytes blocker documented
for full HTTP requests in `docs/upstream-contract.md`; no identity facade is
presented as native execution.
