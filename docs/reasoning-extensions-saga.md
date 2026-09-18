# Reasoning Extensions

Deliver the native capabilities that `../reasoning-from-scratch` requested in
`docs/demo-extensions-requests.md` (work orders E1, E2, and E3): a Hugging Face
`tokenizer.json` byte-level BPE extension (`hftok`), a bounded checksum-verified
large-artifact download over `http-client`, and, only if the upstream `bf16`
dtype slips, an `unpack_bf16` fallback. Every value crosses the boundary as a
string, integer array, record, packed bytes, or opaque handle. Chat templating,
roles, prompts, model semantics, and evaluation stay in MLPL upstream.

For every step, apply the complete `AGENTS.md` completion checklist: scoped and
full pre-commit checks, affected docs, `.gitignore` sanity, tracked source/docs/
configs/AgentRail state, named-file staging, a detailed commit, AgentRail
completion metadata, and a verified `git push origin main`. Work directly on
`main`; do not use feature branches, PRs, `gh`, or GitHub Actions.

Any change needed in a sibling repository is recorded here as a work order for
that repository's agent, in `docs/sw-mlpl-requests.md`,
`docs/demo-mlpl-libraries-requests.md`, or
`docs/reasoning-from-scratch-requests.md`. No sibling repository is modified
from here.

## Revalidation of the work orders (2026-09-16)

The requests document states that nothing in it authorizes a change here and
that the extension agent revalidates each order against the named artifacts
before its saga begins. Findings:

| Order | Already implemented here? | Trigger artifacts | Consequence for this saga |
|---|---|---|---|
| E1 `hftok` | No. No tokenizer, BPE, or `_hftok` code exists in `extensions/`, `crates/`, or `lib/`. | Not yet published: `../reasoning-from-scratch` is on its first saga (`foundation-and-verifier`, step 5 of 8); `fixtures/tokenizer/`, `tests/test_tokenizer_reference.mlpl`, and `fixtures/tokenizer/qwen3-goldens.jsonl` do not exist there. | Build the extension against a synthetic fixture owned here; the upstream parity/throughput step is explicitly gated and reports "unavailable" honestly if the artifacts are still missing. |
| E2 bounded download | No. `extensions/http-client` has a 1 MiB / 10 s `get` and a 16 MiB / 120 s `request`; both buffer the whole body. The streamed, checksum-verified, atomic-rename design exists only as prose in `docs/network-db-extensions.md` and as the archived unstarted step `015-pinned-model-acquisition` (archive `point-cloud-readme-capture-20260911T090810`). | Independent of upstream artifacts; the two Qwen3 sizes are published in the request. | Build first: it unblocks both the 7 MB `tokenizer.json` and the 1.19 GB weights. |
| E3 `unpack_bf16` | No. Only the MLPL vectorized decode evidence in `docs/weight-distribution-blockers.md` mentions bf16. | Upstream `sw-mlpl` still rejects `bf16` in `reinterpret` per `../reasoning-from-scratch/docs/sw-mlpl-blockers.md`; `../reasoning-from-scratch/docs/feature-homes.md` marks the dtype "upstream, in progress". | Decision step late in the saga: implement only if the dtype has not landed. |

Existing conventions reused: the sqlite extension's explicit absolute `root`
plus confined relative `path` record for filesystem confinement; typed
generational handles from the SDK; `ureq` streaming bodies through
`std::io::Read`; `sha2`, `digest`, and `tempfile` already present in
`Cargo.lock`; `fancy-regex` available offline for the lookahead in the Qwen
pre-tokenization pattern (no C `onig` build).

## Steps

1. `http-large-download-core` — Add a pure, testable bounded-download module
   to `extensions/http-client`: a request record with `url`, `expected_bytes`,
   `sha256`, absolute `root`, confined relative `path`, `chunk_bytes`, and
   `timeout_ms`; validation that fails closed on non-canonical roots, traversing
   paths, zero or over-cap lengths, and malformed hex digests; streaming of
   bounded chunks from a `Read` body into a uniquely named temporary file under
   the root with running SHA-256; length and checksum verification; atomic
   rename; and cleanup on every failure. Cover success, truncated, tampered,
   over-declared, `Content-Length` mismatch, redirect, timeout, and existing
   verified-file reuse with loopback listeners only.
2. `http-large-download-surface` — Register `_http:download(record)` through
   the SDK, add the `u:http_download` facade in `module.mlpl`, document the
   contract and ownership split in `docs/network-db-extensions.md`, add a
   loader contract test and an mlplunit test, add a thin `just fetch-artifact`
   recipe with a script, and record an opt-in real-network smoke procedure for
   `Qwen/Qwen3-0.6B-Base/tokenizer.json` (7,031,645 bytes) and
   `model.safetensors` (1,192,135,096 bytes) that writes under an ignored
   `models/` root. Acceptance: a checksum-verified file appears with the
   published size; a truncated or tampered transfer leaves no partial file.
3. `hftok-contract-and-fixture` — Write `docs/hftok-extension.md` (public
   surface, record shapes, root confinement, error taxonomy, what stays in
   MLPL) and scaffold `extensions/hftok` with pure parsing/validation of
   `tokenizer.json`: accept model type BPE with the byte-level pre-tokenizer
   and decoder, reject everything else and malformed JSON with `err`. Publish a
   synthetic `fixtures/tokenizer/tiny-tokenizer.json` in Hugging Face format
   with a small vocabulary, merges, the pre-tokenization pattern, added control
   tokens (`<|endoftext|>`, `<|im_start|>`, `<|im_end|>`, `<think>`,
   `</think>`), and an expected-encodings file; validate the fixture in tests.
4. `hftok-byte-level-bpe` — Implement pure encode and decode: byte-to-unicode
   mapping, pre-tokenization with the file's own pattern via `fancy-regex`,
   merge-rank BPE, added-token and `<|...|>` / think-tag control splitting,
   and the inverse decode that keeps control tokens visible. Tests: every
   expected encoding on the synthetic fixture, byte round-trips on random
   inputs, unknown tokens, and empty input.
5. `hftok-handles-and-facade` — Expose `_hftok:load`, `encode`, `decode`,
   `token_to_id`, and `info` through typed generational handles, `extension.toml`,
   and a `module.mlpl` facade with the requested one-argument `load(path)`
   public signature over the sqlite-style root confinement; return ids as an
   integer dense array and `info` as a record with vocabulary size, the
   end-of-text, turn-start, turn-end, and think ids, and the pattern string.
   Prove stale, wrong-type, and cross-extension handles fail closed; add loader
   contract tests, an mlplunit test through `load_extension`, and a `just`
   check recipe. Record any host gap in `docs/sw-mlpl-requests.md`.
6. `hftok-parity-and-throughput` — Gated on `../reasoning-from-scratch`
   publishing `fixtures/tokenizer/tiny-tokenizer.json`,
   `tests/test_tokenizer_reference.mlpl`, and
   `fixtures/tokenizer/qwen3-goldens.jsonl`. Run the reference suite through the
   extension, round-trip the real Qwen3 goldens, encode the 12,000 training
   prompts, compare with the budget stated in its `docs/plan.md` Saga 2 step 4,
   and record the numbers in `docs/hftok-acceptance.md`. If the artifacts are
   still absent, record an honest "unavailable" result with the synthetic
   evidence and stop without inventing fixtures.
7. `unpack-bf16-fallback-decision` — Re-check whether `../sw-mlpl` accepts
   `bf16` in `reinterpret` (request R7). If it does, record "not needed" with
   evidence and complete without code. Otherwise add a minimal `bf16` extension
   exposing `unpack_bf16(bytes) -> f32 array` with tests for 1, -1, 2, 0.5, 0,
   50, subnormals, infinities, NaN, odd byte counts, and parity with the
   upstream `bf16-vectorized-decode` probe values.
8. `reasoning-extensions-acceptance` — Run the full gate, publish
   `docs/reasoning-extensions-acceptance.md` with per-order status, evidence,
   measured numbers, limitations, and remaining upstream gates; update
   `docs/sagas.md`, `README.md`, and the request documents; mark the saga done.

## Delivery status

| Step | Status |
|---|---|
| 1 `http-large-download-core` | Delivered 2026-09-17 (commit `3ce5b8c`). |
| 2 `http-large-download-surface` | Delivered 2026-09-17. E2 is complete: `_http:download` is registered, the MLPL facade and `just fetch-artifact` exist, and the acceptance criteria are met. |
| 3 `digest-primitive` | Delivered 2026-09-18. Added `extensions/digest` (`_digest:sha256_bytes`, `_digest:sha256_file`) after noticing that E2's checksum verification is unusable by a consumer that cannot compute a digest: SHA-256 is impractical in MLPL, which has no 32-bit integer type for its rounds. Inserted ahead of the tokenizer work because it blocks downstream artifact pinning. See `digest-extension.md` and request D1 in `reasoning-from-scratch-requests.md`. |
| 4 onward | Not started. |

Findings recorded rather than worked around silently: extension calls return a
bare value on success but a result value on failure, so no single MLPL
expression branches on both. Filed as R1 in `sw-mlpl-requests.md`, with R2 for
`get_error` on a string payload. Neither blocks this saga.

Acceptance: the tokenizer extension reproduces the upstream MLPL reference and
real-vocabulary goldens exactly and meets the stated throughput budget or
records the measured shortfall; the download path never leaves a partial or
unverified file under the root; malformed files, unsupported model types, and
stale handles are `err` results and never panics; no Rust code encodes chat
roles, prompts, or model semantics; and every sibling-repository need is
documented rather than implemented from here.
