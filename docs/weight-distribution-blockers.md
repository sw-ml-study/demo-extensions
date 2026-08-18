# Weight-distribution capability and blocker matrix

This matrix separates missing shared model-format work from language/runtime
requirements. It is based on the checked-in contracts in adjacent
`demo-ml-utils`, especially `safetensors_slice.mlpl`, `gguf_slice.mlpl`,
`gguf_q8_0.mlpl`, and their fixtures/tests.

| Input | Current status | Owner of broader support |
|---|---|---|
| Safetensors U8, I8, U16, I16 | Supported through bounded aligned prefix reads | Complete for this slice |
| Safetensors F16, BF16, F32, F64 | Cataloged but deliberately rejected by the selective decoder | `demo-ml-utils`: add finite IEEE decoding, malformed/non-finite policy, bounded fixtures, and statistics adoption |
| GGUF I8 (type 24), I16 (type 25) | Supported through bounded aligned prefix reads | Complete for this slice |
| GGUF Q8_0 (type 8) | Supported as complete 34-byte blocks with binary16 scales | Complete for this slice |
| Standard real-model GGUF metadata arrays | **Correctness resolved by `demo-ml-utils` 31b038f:** bounded scalar arrays and top-level F32/I64/F64 recover the exact SmolLM2 tensor boundary | Correctness is complete; malformed, nested, truncated, and over-budget arrays remain rejected |
| Real-model metadata-array performance | **Resolved by `demo-ml-utils` 3310837:** SmolLM2 traverses 147,209 array elements and 272 tensors in one second at 54,592 KiB peak RSS under its 16 MiB standalone probe; this app adopts lazy `tensor_name_offsets`/`tensor_name_lengths` reads | Complete for catalog memory; the composed interpreter applet still requires its explicit 64 MiB worker-stack ceiling |
| GGUF F16/F32 | Catalog-visible but not selectively decoded by the shared slice/statistics API | `demo-ml-utils`: add aligned finite floating-point selective decode and fixtures |
| GGUF Q4/Q5/K-family and other quantizers | Catalog-visible where type IDs are known, payload decode rejected | `demo-ml-utils`: implement each official block layout, exact extent validation, bounded decode, golden fixtures, and mergeable/sample statistics |
| Reference-vs-quantized error for arbitrary models | Q8_0 error tiles exist only when a caller supplies a matching reference vector | `demo-ml-utils`: define an honest tensor-pair/provenance contract; this app must not infer a reference tensor |
| Representative sampling beyond a prefix | Current app visibly labels a deterministic capped prefix sample | `demo-ml-utils`: provide a reusable seek/stride or reservoir-sampling interchange with offsets, coverage, seed policy, and merge rules |

## sw-MLPL requirements

No new sw-MLPL feature blocks the supported interpreted demo. Bounded
`read_bytes`, filesystem confinement, arrays, records, native ports, retained
scene commands, and parked-main event-loop ownership are already sufficient.

Two existing host gaps remain relevant but do not block this slice:

1. `run_applet_with_host` cannot receive an explicit filesystem root, so this
   repository continues using its documented adapter over public
   `Environment`, `register_port`, and `fs_root`. A configured upstream applet
   entry point would remove that adapter.
2. String-list concatenation is not available on the deployed host surface, so
   Rust injects one bounded combined `.safetensors`/`.gguf` catalog. A generic
   `list_concat` would move multi-pattern discovery entirely into MLPL.

Native compilation remains blocked on the already documented compiler parity
for filesystem I/O, ports/applets, extension startup, and the rendering host.
Those are deployment blockers, not requirements for interpreted histogram
semantics or additional dtype decoders.

The app must continue to show unsupported reasons in its UI, reject before
payload reads, and avoid silently treating raw quantized bytes as scalar
weights. No Rust extension or sw-MLPL builtin should encode model-specific
weight-distribution semantics.

## Real-model acceptance finding

For interactive acceptance, the repo downloaded the ignored Apache-2.0
`SmolLM2-135M-Instruct-Q8_0.gguf` artifact (145 MB published size; local
SHA-256 `bc64cce8e1c11e4ed870633b557e04af718249c817c4cf8a6784116144ec3e28`).
Its Q8_0 tensor payload and standard tokenizer metadata arrays are now
supported after `demo-ml-utils` commit `31b038f`. The shared parser reports 40
metadata entries, five arrays/147,209 elements, 272 tensors, and the exact
1,786,144-byte tensor-data boundary. This application uses a 4 MiB catalog
budget so that validated boundary fits; its former 1 MiB cap correctly failed
closed before payload access. `demo-ml-utils` commit `3310837` consumes the
packed reads and native bounded scanner, retains tensor-name offsets/lengths
instead of padded name matrices, and measures one second at 54,592 KiB peak
RSS under a 16 MiB standalone stack. This app lazily reads only names it shows
or selects. The larger composed interpreter applet still overflows at a 16 MiB
worker stack, so its explicit bounded 64 MiB ceiling remains an application
composition requirement rather than a catalog-memory workaround.
