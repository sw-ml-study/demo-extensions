# Model Atlas architecture metadata and inference

The interactive Model Atlas keeps two different kinds of architectural claims
visibly separate.

- `GGUF ARCH mamba [METADATA]` comes from the fixture's preserved GGUF
  `general.architecture` field and is authoritative for that source artifact.
- `ROLE ... [HEURISTIC]` is inferred in MLPL from the selected tensor name. It
  is a navigation aid, not file metadata or proof of the model graph.

The explicit, deterministic name patterns cover embeddings; attention Q, K,
V, and output projections; normalization; feed-forward projections; MoE gates,
experts, and shared experts; Mamba/SSM projections, convolution, and state; and
output heads. Unrecognized spellings return `UNKNOWN`. Ambiguous names such as
an unqualified `gate.weight`, fused `qk_proj`, and adversarial substrings such
as `expertise` also return `UNKNOWN` rather than guessing.

The checked-in seven-tensor derivative uses representative transformer and
Mamba names so selection demonstrates the classifications. Its sizes remain
tiny fixture values and must not be mistaken for a production model. The
classifier, selected-role display, log-height encoding, filtering, and
schematic layout are MLPL code. Rust receives only generic line geometry,
styles, stable IDs, camera state, and help text.

Future format scanners may preserve additional authoritative metadata through
a versioned interchange update. Heuristic vocabulary can evolve independently,
but any new rule needs positive, unknown, ambiguous, and adversarial mlplunit
fixtures and must remain visibly qualified in the UI.
