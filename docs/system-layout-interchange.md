# System-layout interchange

This repository consumes a bounded, renderer-neutral description of address
spaces. It does not define SWTOS storage semantics. A producer such as
`sw-tos` decides what its regions mean and emits those meanings as data; the
generic Rust code validates the envelope before later MLPL code maps it to
boxes, labels, colors, and interactions.

## Version 1

The canonical schema is [`schemas/system-layout-v1.schema.json`](../schemas/system-layout-v1.schema.json).
The checked-in [`fixtures/system-layout-v1.json`](../fixtures/system-layout-v1.json)
is deliberately synthetic and is not an SWTOS image.

Each layout has:

- `schema = "sw-ml-study.system-layout"` and `version = 1`;
- provenance naming the producer, its exact revision, generation time, and a
  human-readable source description;
- one or more bounded address `spaces`, each with a stable ID, unit, extent,
  and optional block and sector hints;
- bounded `regions` with stable IDs, a space reference, unsigned address and
  extent, and producer-defined `purpose`, `owner`, `location`, and `state`;
- bounded directed `relationships` between region IDs, with a producer-defined
  kind; and
- optional `C24IMG` composition metadata identifying a zero-based part within
  a nonempty multi-part image.

IDs are opaque case-sensitive UTF-8 strings. Consumers must not infer SWTOS
meaning from an ID or attribute vocabulary. Addresses and extents use unsigned
64-bit integers in the declared space unit. A region must be nonempty, its
addition must not overflow, and its half-open interval `[address, address +
extent)` must fit its space. Block and sector sizes are layout hints, not
renderer dimensions; when present they must be nonzero divisors of the space
extent. Version 1 does not forbid overlapping regions because overlays and
runtime views may intentionally describe the same bytes.

## Validation and compatibility

Parsing owns all accepted values and performs no I/O. Unknown JSON fields and
wrong JSON types are malformed. Semantic validation returns the first error in
this order: schema, version, collection budgets, provenance text, spaces,
regions, relationships, composition. Within a collection, source order is
used. Limits are 64 spaces, 65,536 regions, 131,072 relationships, and 1,024
UTF-8 bytes per text value. Text must be nonempty and contain no control
characters. IDs must be unique within their own collection, references must
resolve, relationship endpoints must differ, and composition requires
`part_index < part_count`.

Consumers accept exactly version 1. Additive fields therefore require version
2 rather than being silently ignored. Producers should retain stable IDs across
snapshots when an entity is logically unchanged. A producer revision records
provenance, not protocol compatibility. Malformed or unsupported documents are
rejected atomically before scene construction; there is no partial result or
best-effort rendering.

## Cross-repository handoff

`sw-tos` should publish its authoritative generated artifact and a pinned
producer revision without copying its domain rules into this crate. Once that
artifact is available, a later saga step will vendor or hash-pin it and prove
that an MLPL adapter drives the generic renderer. `sw-mlpl` changes, if needed
for live events or extension transport, remain separately owned upstream.
