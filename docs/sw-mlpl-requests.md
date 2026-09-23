# Requests to `../sw-mlpl`

Changes this repository needs from the `sw-mlpl` host. Nothing here is
implemented from this repository; each entry is a work order for the upstream
agent, revalidated against the named artifacts before that agent acts.
Historical host-contract gaps and their resolution status remain in
`upstream-contract.md`; this file holds only open requests raised by the
`reasoning-extensions` saga (`reasoning-extensions-saga.md`, complete
2026-09-22). Both were re-checked at `sw-mlpl` `dd776fff`.

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

Status: open, not blocking. It still reproduces at `dd776fff`. This repository
ships the `type_of` workaround and documents it.

## R2. `get_error` on a string payload

Raised by: step `002-http-large-download-surface` (2026-09-17).

`get_error(result)` on an err carrying a string message fails with
"boxing a non-scalar payload (string) as an Option needs Stage 6 enclose; use
unwrap/err_message instead". `err_message` works and is what this repository
uses. Recorded so the upstream agent can decide whether Stage 6 enclose or a
narrower `get_error` contract is the intended resolution.

Status: open, still reproduces at `dd776fff`, worked around with
`err_message`.

Not listed here: bulk `unpack(bytes, dtype)`. That is
`../reasoning-from-scratch`'s request R11, filed from that repository. This
saga's step 9 decision depends on it (`reasoning-extensions-saga.md`, "E3
decision").
