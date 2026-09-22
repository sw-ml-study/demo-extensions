# Requests to `../reasoning-from-scratch`

Changes this repository needs from its downstream consumer. Nothing here is
implemented from this repository; each entry is a work order for that
repository's agent, who revalidates it before acting. This file exists because
work orders flow both ways: that repository's asks of this one live in its
`docs/demo-extensions-requests.md`.

## D1. Pin artifact digests, now that a SHA-256 primitive exists

Raised by: step `003-digest-primitive` (2026-09-18).

`scripts/fetch-math500` and `scripts/fetch-model` verify a download by
published byte size alone. A size check detects a truncated transfer but not a
substituted one: any replacement of the same length passes.

Two capabilities now exist here to close that gap:

- `_digest:sha256_bytes` / `_digest:sha256_file` compute a digest natively,
  streaming a file in bounded chunks. See `digest-extension.md`. This exists
  because SHA-256 is impractical to implement in MLPL, which has no 32-bit
  integer type for its rounds.
- `_http:download` streams a transfer into a temporary file under a granted
  root, verifies a declared length and SHA-256, and only then renames it into
  place. See `network-db-extensions.md`.

Requested: obtain the published SHA-256 for each pinned artifact, record it
alongside the byte size already recorded, and verify against it. The artifacts
named in `demo-extensions-requests.md` E2 are
`Qwen/Qwen3-0.6B-Base/tokenizer.json` (7,031,645 bytes) and
`model.safetensors` (1,192,135,096 bytes); `MATH-500/test.jsonl` (446,564
bytes) has the same gap.

Note for whoever does this: Hugging Face serves large files through Git LFS,
and the LFS pointer carries the SHA-256. Requesting the raw pointer rather than
the resolved file yields the digest in a few hundred bytes, with no need to
transfer 1.19 GB to learn it.

Why this is not done here: the digests are provenance for artifacts that
repository pins, licenses, and documents. Choosing and recording them is its
decision, and this repository does not modify sibling repositories.

Status: open. It does not block any step of the active saga here.

## D2. Integrate the Linux tokenizer and download packages

Requested 2026-09-22. Linux release packages now exist locally for `hftok`
with NFC and `http-client` with checksum-verified downloads. Rebuild commands,
exact package paths, the required host version, real Qwen3 smoke evidence,
and limitations are in [reasoning-linux-delivery.md](reasoning-linux-delivery.md).
The download Linux link failure from duplicate extension entry symbols is
fixed. Use the public facades and pinned artifact sizes and hashes in the
consumer's real-model fetch workflow.

Status: downstream integration pending. `../reasoning-from-scratch` is absent
on this machine, so no consumer files or tests were changed. Its reference
parity and throughput artifacts remain required for the gated acceptance step.
