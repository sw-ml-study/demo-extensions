# Signature Metadata

Extension ABI V1 carries rich signatures as one bounded UTF-8 TOML document.
This keeps the raw C boundary to a single pointer/length pair; nested argument,
default, and documentation structures are parsed only from a host-owned copy in
safe Rust.

## Schema

Each `[[functions]]` entry declares `name`, `documentation`, `returns`, and
ordered `[[functions.arguments]]` entries. Arguments declare `name`, `type`,
and an optional typed `default`. Each `[[types]]` entry declares an
extension-defined native type name and its documentation.

```toml
[[functions]]
name = "greet"
documentation = "Return a greeting count."
returns = "i64"

[[functions.arguments]]
name = "name"
type = "string"
default = "world"

[[functions.arguments]]
name = "excited"
type = "bool"
default = false

[[types]]
name = "Greeting"
documentation = "An opaque greeting resource."
```

Scalar defaults are checked against `bool`, `i64`, `f64`, and `string`.
Integers are accepted as exact defaults for `f64`; bytes, nil, arrays, and
native types currently have no TOML default encoding.

## Fail-closed registration

Parsing rejects malformed TOML, duplicate functions, duplicate arguments,
duplicate native types, and incompatible defaults. The loader then compares
the metadata with copied ABI descriptors. Both name sets must be identical and
each arity must equal its ordered metadata argument count. Mismatch diagnostics
sort name lists so output does not depend on provider order.

Help preserves declared argument order and renders a stable signature followed
by documentation. Native-type help is similarly deterministic. Unknown names
return typed errors; there is no best-effort or partially registered state.

## Boundary and upstream status

The ABI validator caps and copies the metadata slice before the SDK parses it.
Extensions must keep raw descriptor storage readable only for validation; the
registry retains the parsed host-owned model.

This repository exposes help through its Rust registry harness. Making the same
metadata available to MLPL `help`, imports, the REPL, or compiled programs
still requires the public registry/import/compiler hooks listed in
[the upstream contract](upstream-contract.md). No `../sw-mlpl` change is made
by this step.
