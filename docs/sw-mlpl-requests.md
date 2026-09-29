# Requests to `../sw-mlpl`

Changes this repository needs from the `sw-mlpl` host. Nothing here is
implemented from this repository; each entry is a work order for the upstream
agent, revalidated against the named artifacts before that agent acts.
Historical host-contract gaps and their resolution status remain in
`upstream-contract.md`; this file holds requests raised by the `reasoning-extensions`
saga (`reasoning-extensions-saga.md`).

**No requests are open.** Both entries below were resolved by `sw-mlpl`
`b3180d9a` ("bulk unpack(bytes, dtype); is_result(); get_error message") and
re-probed on 2026-09-29 with the release `mlpl-repl` 0.22.0 selected by
`scripts/select-mlpl`. Upstream's account is in `../sw-mlpl/docs/q-and-a.md`
and `docs/downstream-updates.md`.

## R1. Symmetric result shape from extension calls

Raised by: step `002-http-large-download-surface` (2026-09-17).

An extension function that returns `Ok` yields a **bare value** to MLPL, while
one that returns `Err` yields a **result value**. Observed with
`_http:download`, which returns a record on success and an err result on
failure:

```mlpl
result = _http:download(request);
is_ok(result)      # hard error on success: "expected a Result value, got record"
result.path        # hard error on failure: "requires a record receiver, got result"
```

Neither `is_ok`/`is_err` nor field access is total across both outcomes, so no
single expression can branch on the result of an extension call. The demo in
`demos/http-client/download.mlpl` works around this by classifying with
`type_of(result)` and comparing with `str_eq`, which is correct but obscures
the intended railway pattern and breaks if a call ever legitimately returns a
result value on success.

Requested: extension calls should return a result value in both directions, so
`is_ok`, `unwrap`, `err_message`, and postfix `?` apply uniformly. If the bare
success value is deliberate for ergonomics, then a total predicate
(`is_result`, or `is_ok` returning false rather than raising for a non-result)
would also resolve it.

Impact: every extension consumer is affected. The `hftok` facade and the
consumer's parity runner branch on `type_of`, because `is_ok` on a successful
`_hftok:load_path` raises "expected a Result value, got ext-handle".

Status: **resolved by `is_result(x)`**, a total predicate that never raises.
Upstream chose not to wrap every extension success in `ok(...)`, because that
would break existing consumers. `is_ok` still requires a Result, since
answering 0 for a bare success would read as a failure. Re-probed:
`is_result` answers 0 for a successful `_hftok:load_path` handle, 1 for a
failed load, and 0 for `7`. The supported branch is
`if is_result(r) { err_message(r) } else { ... }`.

This repository's MLPL still uses the older `type_of` classification. It is
correct on every host, and migrating to `is_result` would raise the minimum
host to `b3180d9a`. That migration is deliberately deferred.

## R2. `get_error` on a string payload

Raised by: step `002-http-large-download-surface` (2026-09-17).

`get_error(result)` on an err carrying a string message fails with
"boxing a non-scalar payload (string) as an Option needs Stage 6 enclose; use
unwrap/err_message instead". `err_message` works and is what this repository
uses. Recorded so the upstream agent can decide whether Stage 6 enclose or a
narrower `get_error` contract is the intended resolution.

Status: **resolved as a documented boundary.** The Option form of `get_error`
holds scalar payloads only, and `err_message(r)` is the supported way to read
a string error. The error now says so: "the Option form holds scalar payloads
only; this payload is a string. Read it directly with err_message(r)". This
repository already uses `err_message`, so nothing changes here.

Related, not filed from here: bulk `unpack(bytes, dtype)`,
`../reasoning-from-scratch`'s request R11, also shipped in `b3180d9a`. It
settles this saga's E3 decision (`reasoning-extensions-saga.md`, "E3
decision").
