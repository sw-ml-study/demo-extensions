# Bounded Model Atlas Scanning

Model Atlas does not read a complete model file or tensor payload into memory.
The scanner uses `file_size` and caller-capped `read_bytes(path, offset,
length)` operations. It makes two deterministic passes over explicitly listed
catalog ranges, retains compact tensor columns, and reads payload bytes only
for a selected detail request.

## Adapter boundary

Format adapters produce schema `sw-ml-study.model-atlas.scan-adapter`, version
1. The record identifies `safetensors` or `gguf`, the observed file size,
`catalog_ranges:[N,2]`, and a columnar `tensor_columns` record:

- count, offsets, lengths, ranks, padded shapes, and adapter-defined dtype
  codes;
- UTF-8 name lengths and a fixed-width padded name-byte table.

The scanner deliberately does not duplicate the accepted Safetensors and GGUF
parsers in `demo-ml-utils`. Those parsers can adapt their bounded catalog IR to
this columnar boundary. The next interchange-contract step will pin checked-in
derived fixtures and provenance between repositories. This repository's small
fixture tests the generic scanner rather than claiming full format parsing.

Both catalog passes reread each range and compare fingerprints. Tensor payload
ranges are descriptors only: they are validated for exact-integer bounds,
catalog overlap, tensor overlap, rank and parameter-product overflow, but are
not read during scanning. Multiple passes therefore trade bounded I/O for low
retained memory instead of caching a model-sized buffer.

## Budgets and on-demand detail

Callers explicitly cap catalog bytes, tensor count, padded rank, name width,
iterations, bytes per detail read, cache bytes, and cache entries. A detail
request is relative to one validated tensor range and fails before I/O if it
escapes that tensor or exceeds its budget. Cache admission separately checks
both retained bytes and entry count. The current value returned by
`atlas_read_detail` owns one bounded byte array; zero-copy is not claimed.

Files above MLPL's exact non-negative integer domain (`2^53-1`) are rejected.
Format-specific semantics—including GGUF alignment/type interpretation and
Safetensors header decoding—remain adapter responsibilities. This slice does
not decode floating-point or quantized tensor values and does not infer model
architecture.

## Measured sparse-file evidence

On 2026-08-16, `just model-atlas-memory-evidence` scanned temporary 1 MiB and
64 MiB sparse files. Each run reread two 8-byte catalog passes, retained one
descriptor, and fetched one 4-byte selected detail:

| File size | Peak RSS |
|---:|---:|
| 1 MiB | 9,060,352 bytes |
| 64 MiB | 9,076,736 bytes |

The 64-fold file growth increased observed peak RSS by 16,384 bytes, below the
8 MiB growth ceiling; both stayed below the 48 MiB absolute ceiling, and the
64 MiB artifact itself exceeded that ceiling. This supports memory driven by
fixed scan/detail state rather than total file length on the measured arm64
macOS host. The script supports Darwin and Linux `/usr/bin/time` formats, but
this evidence is not a Linux visual or RSS run.
