# Model Atlas bounded detail views

The derived `just model-atlas` city attaches bounded drill-downs to two
representative tensors. The Safetensors embedding shows summary statistics, a
four-bin cyan histogram, and a four-point green sampled surface. The GGUF
Mamba projection adds four red Q8 reconstruction-error marks plus RMSE,
maximum-error, and cosine metrics.

The fixture uses the renderer-neutral schema names
`sw-ml-study.distribution-surfaces` and `sw-ml-study.q8-error-tile`, compatible
with `demo-ml-utils`. Its provenance states `bounded synthetic values; not user
model payload`. Other selections display `PAYLOAD DETAIL UNSUPPORTED`; no
fallback reads arbitrary payload data.

## Bounds and retained updates

The fixture permits at most four histogram bins, four surface points, four
error points, absolute coordinates of 128, and 16 detail objects. Stable detail
IDs start at 5000. Validation fails before scene construction if a schema or
budget is wrong.

The base city has 165 lines. Selecting the supported Safetensors detail upserts
12 selected-building lines plus seven detail lines: 19 updates, or 11.5% of
the base scene. Moving from that detail to an unsupported tensor upserts only
24 old/new building lines and removes seven detail IDs. MLPL never resends the
complete city for these selections.

On the 2026-08-16 macOS development checkout, the complete 11-test Model Atlas
mlplunit suite finished in 2.05 seconds wall time (`user 1.56`, `sys 0.07`).
This is local structural evidence, not a cross-platform latency guarantee.

MLPL owns schemas, budgets, provenance, statistics, visual mappings,
unsupported behavior, and patch construction. Rust owns generic command
validation, retained line IDs, native events/windowing, and rendering. The
real-file picker still reads only its bounded Safetensors header; real payload
detail must later use bounded decoders from `demo-ml-utils`.
