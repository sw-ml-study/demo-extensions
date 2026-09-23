# `hftok` parity and throughput acceptance

Step `008-hftok-parity-and-throughput`, measured 2026-09-22 on macOS against
`../reasoning-from-scratch` at commit `8cef001`. That repository was read,
never modified: its worktree was clean before and after every run.

## Result

| Criterion (work order E1) | Result |
|---|---|
| 1. Reference suite ids through the extension | **Passed on the published expectations.** The named suite `tests/test_tokenizer_reference.mlpl` does not exist upstream; see below. |
| 2. Qwen3 goldens round-trip on the real vocabulary | **Passed**, 8 of 8 encode to the golden ids and decode to the golden text. |
| 3. 12,000 training prompts within the stated budget | **Unavailable as specified.** The corpus is not fetched and no numeric budget is stated. A proxy measurement is recorded below and is not a substitute. |
| 4. Malformed files, unsupported types, stale handles are `err` | Covered by earlier steps' contract tests (`tokenizer_file_contract.rs`, `provider_contract.rs`), which remain green. |

No parity defect was found, so no encoder code changed in this step.

## Artifacts checked

| Upstream artifact | State |
|---|---|
| `fixtures/tokenizer/tiny-tokenizer.json` | present |
| `fixtures/tokenizer/tiny-expected.jsonl` | present, 6 cases |
| `fixtures/tokenizer/qwen3-goldens.jsonl` | present, 8 cases |
| `models/qwen3-0.6b-base/tokenizer.json` | present, 7,031,645 bytes, SHA-256 `c0382117ea329cdf097041132f6d735924b697924d6f6fc3945713e96ce87539` |
| `tests/test_tokenizer_reference.mlpl` | **absent** |
| `data/math-train/` (12,000 training prompts) | **absent**; upstream plans `just fetch-math-train` for a later step |
| numeric budget in `docs/plan.md` Saga 2 step 4 | **absent**; the step says "under a stated time budget" without stating one |

The real vocabulary was fetched by the consumer's own `just fetch-model`, which
checks the byte size only (request D1 in `reasoning-from-scratch-requests.md`
asks it to pin the digest recorded above). The work order names
`just fetch-artifact`; that recipe exists here but was not needed, since the
file was already present.

In place of the absent reference suite, the consumer's reference encoder
(`lib/tokenizer/bpe.mlpl`) is tested upstream by `tests/test_tokenizer_bpe.mlpl`,
and its committed outputs are `tiny-expected.jsonl` and the goldens, which is
what both checks below compare against.

## Evidence

### Rust parity suite

`extensions/hftok/tests/upstream_parity.rs` reads the upstream files in place.
Each test prints `SKIP` and passes when its artifact is absent, so the gate
stays green on machines without the consumer checkout; it never invents a
fixture.

```text
$ cargo test -p mlpl-extension-hftok --test upstream_parity -- --nocapture
checked 6 upstream tiny expectations
merge-rank invariant: 151387 merges, 0 mismatches
checked 8 Qwen3 goldens
test result: ok. 4 passed; 0 failed
```

The merge-rank test confirms the invariant the work order says the extension
may rely on: for all 151,387 merges, the merged token's id is its rank plus
256. This encoder still uses an explicit rank table and does not depend on it.

### Consumer's own MLPL parity runner

`../reasoning-from-scratch/scripts/run-tokenizer-parity` loads the release
`libmlpl_extension_hftok.dylib` through `load_extension` and compares its
output with the MLPL reference encoder on the fixture, and with the goldens on
the real vocabulary:

```text
loaded extension namespace: _hftok
   fixture expectations : 6 of 6 matched
     reference ms: 2  extension ms: 0
   real vocabulary goldens : 8 of 8 matched
Ok(6)
```

The real-vocabulary load, refused before step 7 because the file declares an
NFC normalizer, now succeeds.

### Throughput (proxy only)

`extensions/hftok/examples/throughput.rs` encodes the `problem` field of each
JSON Lines row, then decodes every result and compares it with the NFC form of
the input, failing if any prompt does not round-trip. Timings cover the Rust
encoder in-process: no MLPL interpreter or ABI crossing is included.

```sh
just hftok-throughput <tokenizer.json> <corpus.jsonl> [prompt-count]
```

Corpus: MATH-500 `data/math500/test.jsonl` (500 distinct problems, 97,946
bytes), cycled to 12,000 prompts (2,350,704 bytes, 851,280 tokens).

| Run | Load ms | Encode ms, 12,000 prompts | Round-trip misses |
|---|---|---|---|
| 1 | 176.0 | 1425.9 | 0 |
| 2 | 145.7 | 824.2 | 0 |
| 3 | 142.3 | 829.6 | 0 |
| 4 | 141.2 | 1083.8 | 0 |

A single uncycled pass over the 500 problems took 36.2 ms (72 µs per prompt).

Hardware and build: Apple M1 Max, 10 cores, 64 GB, macOS 26.5, rustc 1.96.0,
`--release` profile, one thread, wall-clock `Instant`, no warm-up, on an
ordinary desktop session rather than an isolated benchmark host.

Limitations of this measurement:

- It is a proxy. MATH-500 is a test split from the same distribution as the
  training problems, but it is not the 12,000-prompt corpus, and cycling
  500 prompts may keep caches warmer than 12,000 distinct ones would.
- The prompts are raw problems without the chat template, which adds a few
  control tokens per prompt upstream.
- Without a stated budget there is nothing to pass or fail. For scale, the
  consumer's MLPL reference took 1.3 to 6.7 seconds for *each* one-word golden
  (`encode_ms` in `qwen3-goldens.jsonl`).

## Remaining gates

To close criterion 3 as written, upstream needs to publish the training split
(`just fetch-math-train`) and a numeric budget; then rerun
`just hftok-throughput` on that corpus with no prompt count. Both belong to
`../reasoning-from-scratch` and are noted in request D2. If upstream later
publishes `tests/test_tokenizer_reference.mlpl`, `run-tokenizer-parity` is the
place to run it through the extension.

Upstream's `docs/demo-extensions-requests.md` E1 still describes the NFC
refusal as the open gap. That is now out of date; updating it is that
repository's decision.
