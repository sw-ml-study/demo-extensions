# Reasoning extensions: saga acceptance

Final report for the `reasoning-extensions` saga (steps 001–010), 2026-09-22.
The saga answered the work orders in
`../reasoning-from-scratch/docs/demo-extensions-requests.md`. Plan, per-step
status, and the step 9 decision record: `reasoning-extensions-saga.md`.

Evidence was gathered against `../reasoning-from-scratch` at `8cef001` and
`../sw-mlpl` at `dd776fff` (`mlpl-repl` 0.22.0 release build), on macOS
(Apple M1 Max, rustc 1.96.0). Linux x86_64 evidence is from step 7
(`reasoning-linux-delivery.md`). No sibling repository was modified.

## Per-order status

| Order | Status | Where |
|---|---|---|
| E1 `hftok` tokenizer | **Delivered.** Parity with the upstream reference passes. The throughput criterion is unavailable as written because upstream artifacts are missing. | `extensions/hftok`, `hftok-extension.md`, `hftok-acceptance.md` |
| E2 bounded verified download | **Delivered** and acknowledged upstream as "exactly the shape that was asked for". | `extensions/http-client`, `network-db-extensions.md` |
| E3 `unpack_bf16` fallback | **Not built, by decision.** The `bf16` dtype landed in `sw-mlpl`, and upstream has not raised E3. | `reasoning-extensions-saga.md`, "E3 decision" |
| (unrequested) SHA-256 primitive | **Delivered**, because a consumer cannot supply E2's checksum without it. | `extensions/digest`, `digest-extension.md` |

## E1: `hftok`

Delivered behavior: `_hftok:load`, `load_path`, `encode`, `decode`,
`token_to_id`, `info`, `close`, and `validate` behind typed generational
handles, plus a `module.mlpl` facade (`u:hftok_load`, `u:hftok_encode`, …).
It implements byte-level BPE using the file's own pre-tokenization pattern,
merges, and added tokens, with NFC normalization. Chat templates, roles, and
end-of-sequence rules stay in MLPL upstream.

Evidence:

| Criterion | Result |
|---|---|
| 1. Reference ids identical | 6 of 6 published fixture expectations match, both in Rust (`tests/upstream_parity.rs`) and through the consumer's own `scripts/run-tokenizer-parity` via `load_extension`. |
| 2. Qwen3 goldens round-trip | 8 of 8 encode to the golden ids and decode to the golden text, on the real 7,031,645-byte vocabulary. |
| 3. 12,000 prompts within budget | Unavailable as specified; see limitations. Proxy: MATH-500 cycled to 12,000 prompts (851,280 tokens) encodes in 0.82–1.43 s across four release runs, with 0 round-trip misses. |
| 4. Errors, never panics | Contract tests cover malformed files, unsupported model types, stale, foreign, and wrong-type handles, and malformed arguments. |

Also checked: the rank+256 merge invariant holds for all 151,387 Qwen3
merges. The encoder does not depend on it. On Linux, the release `.so` loaded
in a current host encoded composed and decomposed `Café` to identical ids.

Scoped tests: 36 pass in `mlpl-extension-hftok`, plus the mlplunit suite
`tests/test_hftok_module.mlpl` and `just hftok-check`.

## E2: bounded large-artifact download

Delivered behavior: `_http:download(record)` and the facade
`u:http_download(url, expected_bytes, sha256, root, path, chunk_bytes,
timeout_ms)`. The body streams into a temporary file beneath an explicit
root. The download verifies the declared length and SHA-256, `fsync`s, and
renames atomically. A file that is already present and verified is reused,
and nothing unverified survives a failure. `just fetch-artifact` is the thin
command-line entry.

Evidence: 21 scoped tests pass in `mlpl-extension-http-client`, using
loopback listeners only. They cover truncation, tampering, over-delivery,
length mismatch, bad status, redirects, stalls, reuse, stale replacement, and
pre-I/O rejection. Real network: on Linux, the Qwen3 `tokenizer.json`
downloaded and verified against SHA-256
`c0382117ea329cdf097041132f6d735924b697924d6f6fc3945713e96ce87539`, and a
second call reused it without a transfer. The same digest was measured on
macOS for the consumer's independently fetched copy.

The digest primitive (`_digest:sha256_bytes`, `_digest:sha256_file`) has
7 scoped tests and the mlplunit suite `tests/test_digest_module.mlpl`.

## E3: bf16 fallback

Checked at `sw-mlpl` `dd776fff`. `reinterpret` accepts `bf16` and `f16`, and
`read_bf16_le` decodes correctly. There is no bulk `unpack`. That gap is the
consumer's own request R11 to `sw-mlpl`, which `sw-mlpl` has deferred as
"non-critical", not declined. The consumer's E3 says to build nothing until
R11 is declined, and names a `sten:read_tensor` reader as the fallback shape.
No code was written.

## Gate

`just check` passes on this final commit. It covers workspace Rust tests,
clippy, formatting, `scripts/check-mlpl-style`, upstream-host tests, native
mlplunit, and the headless integration checks.

## Limitations

- **Throughput criterion 3 is not closed.** The 12,000-problem training split
  is not fetched upstream, and `docs/plan.md` Saga 2 step 4 states no numeric
  budget. The MATH-500 proxy is in-process Rust timing only: it excludes the
  interpreter and ABI crossing, and it cycles 500 prompts.
- **`tests/test_tokenizer_reference.mlpl` does not exist upstream.** Parity
  used the published expectation files, which are the reference encoder's
  committed outputs.
- **Library names.** Built libraries are named
  `libmlpl_extension_<name>.*`, so the consumer loads them by path, not by bare
  name.
- **Result shape and `get_error`.** Extension results are bare on success and
  Result-shaped on failure (R1), and `get_error` fails on string payloads
  (R2). Both reproduced at `dd776fff`; consumers branch on `type_of` and use
  `err_message`.
- **NFC is lossy by design.** Decoded text is normalized, so decomposed input
  does not round-trip byte for byte. NFC files whose added tokens are marked
  normalized are refused.
- **Download.** It has no resume, no parallel ranges, and no progress
  reporting. `timeout_ms` is a whole-transfer deadline. The weights'
  1.19 GB transfer has not been exercised here.
- **Platforms.** macOS arm64 and Linux x86_64 were measured. Linux aarch64 was
  not. Built libraries are local artifacts, not published binaries.

## Remaining upstream gates

| Gate | Owner | Unblocks |
|---|---|---|
| Publish the training split and a numeric encoding budget | `reasoning-from-scratch` | closing E1 criterion 3 (`just hftok-throughput`) |
| Update E1's status in its request document (NFC is resolved) | `reasoning-from-scratch` | accurate consumer records |
| Pin artifact SHA-256 digests (D1) and integrate the packages (D2) | `reasoning-from-scratch` | verified model fetches upstream |
| R11 bulk `unpack(bytes, dtype)` | `sw-mlpl` | the consumer's weight loading, and deciding whether E3 is ever raised |
| R1 symmetric extension result shape, R2 `get_error` on strings | `sw-mlpl` | uniform railway handling of extension calls |

None of these is work for this repository until the owning repository acts.
