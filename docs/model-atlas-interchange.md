# Model Atlas Interchange Contract

The Model Atlas handoff consumes `sw-ml-study.tensor-city` version 1, the
renderer-neutral IR already produced by `demo-ml-utils`. Transport is generic
tagged JSON so numeric arrays, records, and strings survive a deterministic,
budgeted round trip. `just model-atlas-contract` validates and summarizes the
checked-in cross-format derivative.

## Checked-in derivative and provenance

`fixtures/model-atlas/tensor_city_derived.mlpl` reproduces the accepted
`demo-ml-utils` golden catalog result: three Safetensors tensors and four GGUF
tensors. It preserves source artifact IDs, format labels, analyzer identity,
stable tensor IDs, names, parameter and encoded-byte counts, hierarchy depths,
districts, and deterministic geometry. It contains no tensor payload bytes.

This is a derived compatibility fixture, not an independent Safetensors or
GGUF parser and not a claim that the neighboring repository is locked to this
checkout. A later integration update can regenerate/publish the same versioned
IR from `demo-ml-utils`; changes require an explicit schema-version or
compatibility decision rather than silent positional reinterpretation.

## Required shape

The root contains exactly `schema`, `version`, `provenance`, and
`tensor_columns`. Provenance names the analyzer and exact Safetensors/GGUF
source records. Tensor columns contain:

- fixed-width UTF-8 name and stable-ID tables plus their lengths;
- format, artifact-group, and name-hierarchy codes;
- x/y/z centers and width/depth/height extents;
- parameter and encoded-byte counts.

For version 1, stable IDs are `<artifact-id>:tensor:<tensor-name>`.
Safetensors is format/group 0 at z=0 and GGUF is format/group 1 at z=4. Within
each district x advances by two, width/depth are one, height is
`1 + parameter_count`, and y is half-height. These rules pin the existing
tensor-city layout rather than allowing a renderer to guess.

## Validation and bounds

MLPL rejects unknown/missing fields, schemas or versions, provenance/format
mismatches, misaligned columns, malformed padded tables, invalid or colliding
stable IDs, nondeterministic layout, and excess tensor, logical-object,
hierarchy, coordinate, parameter, element, iteration, JSON depth/element, or
encoded-byte budgets. Repeat tagged encoding is byte deterministic in the
golden test.

`demo-ml-utils` owns bounded file-format parsing, tensor discovery, source
offset interpretation, and tensor-city construction. This repository owns
interchange validation, native layout adaptation, interaction, and generic
rendering. Architecture classification remains a later MLPL step and must
distinguish metadata from name-based inference. Rust receives generic scene
objects only and contains no Safetensors, GGUF, model, or tensor semantics.
