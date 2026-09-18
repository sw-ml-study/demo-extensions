# Content-digest extension (`_digest`)

A generic SHA-256 primitive. It hashes bytes; it knows nothing about models,
vocabularies, artifact names, or why a caller wants a digest.

## Why this is native

SHA-256 compresses each 64-byte block through 64 rounds of 32-bit modular
addition, bitwise rotation, and exclusive-or. The 1.19 GB artifact this
repository's consumers pin is roughly 18.6 million blocks, so about 1.2 billion
round operations.

MLPL has no native 32-bit integer type. Expressing the rounds means emulating
`u32` wrap-around, rotation, and xor on f64 arrays, which multiplies both the
operation count and the memory traffic. The result would not be a slow
implementation of a fast algorithm; it would be an impractical one, and the
digest of a gigabyte artifact would never finish in a teaching demo.

By this repository's own rule in `../reasoning-from-scratch/docs/feature-homes.md`,
that makes it extension work: no differentiation, byte-level, throughput-bound,
and shipped in a standard library everywhere else.

## Surface

Private namespace `_digest`, public facade `digest` in
`extensions/digest/module.mlpl`.

| Call | Returns |
|---|---|
| `_digest:sha256_bytes(bytes)` | lowercase hexadecimal digest string |
| `_digest:sha256_file({root, path, chunk_bytes})` | record with `sha256` and `bytes` |

`u:digest_sha256_bytes(bytes)` and `u:digest_sha256_file(root, path,
chunk_bytes)` are the public facade calls, with
`u:digest_sha256_file_request(...)` building the record alone so tests can pin
its shape without reading anything.

`sha256_file` confines exactly as the SQLite and download surfaces do: `root`
must be an absolute existing directory, `path` a non-empty relative path whose
components are all normal, and the canonical target must still resolve beneath
the canonical root. A symbolic link pointing outside the root is rejected after
canonicalization, not before. `chunk_bytes` runs from 1 to 16 MiB and bounds
resident memory, so hashing a gigabyte file never loads a gigabyte.

## Error taxonomy

Invalid-argument errors, raised before any read: a non-record or non-exact
record, a non-bytes argument to `sha256_bytes`, a relative or missing root, an
empty, absolute, or traversing path, a path that escapes the root or does not
name a regular file, and an out-of-range `chunk_bytes`. Extension errors cover
a file that cannot be opened or read. Nothing panics.

## One hashing implementation

`Sha256Stream` in `extensions/digest/src/sha256.rs` is the only SHA-256
implementation in the repository, including its hexadecimal rendering. The
bounded download in `http-client` drives that type directly from its own read
loop, because it must also enforce a byte limit and tee each chunk into a
temporary file. Transfer policy stays in the download; hashing stays here.

## Evidence

`extensions/digest/tests/digest_contract.rs` checks the published NIST
one-block and two-block vectors and the empty-input vector, agreement between
the in-memory and streamed file digests at chunk sizes of 1, 64, 4096, and
1 MiB over a 4,097-byte input whose length is deliberately not a multiple of
any of them, an empty file, symbolic-link escape, and every fail-closed input.
`extensions/digest/tests/provider_contract.rs` pins the registered names, help
text, and argument rejection across the dynamic and static providers.
`tests/test_digest_module.mlpl` pins the facade record shape.

## What this unblocks

Consumers that pin an artifact by digest can now compute and verify one without
the algorithm in MLPL. `../reasoning-from-scratch` currently verifies its
downloads by byte size alone; a size check detects truncation but not
substitution. With this primitive its fetch recipes can pin a real digest, and
`u:http_download` can consume it.

Neither this repository nor its consumers hold published SHA-256 values for the
Qwen3 artifacts yet. Obtaining and pinning them is the consumer's decision and
belongs in that repository, not here.
