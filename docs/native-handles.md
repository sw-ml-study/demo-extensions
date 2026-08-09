# Typed Generational Native Handles

Native resources remain inside an extension-owned `HandleRegistry`. MLPL-facing
values carry only four integers: extension identity, type identity, slot, and
generation. The ABI never exposes a resource pointer, Rust object address, or
process-global table index.

## Validation and reuse

Every lookup and removal validates, in order, registry activity, extension
identity, declared type identity, slot bounds, generation, live occupancy, and
the stored Rust type. Failures are typed as inactive, exhausted, wrong
extension, wrong type, or stale.

Removing a resource invalidates its capability before returning the object. A
reusable slot advances its generation. When its configured generation limit is
reached, the slot retires permanently rather than wrapping and making an old
capability valid again. Allocation uses the lowest reusable slot, then appends
within the configured capacity; exhausted registries fail closed.

## Finalization and deactivation

Explicit removal returns ownership to the extension caller for immediate
finalization. Registry deactivation first makes every operation inactive, then
drops live resources in ascending slot order. Dropping the registry applies the
same idempotent deactivation path. Tests observe multi-resource drop order and
cover stale reuse, wrong Rust and declared types, cross-extension use, slot and
generation exhaustion, and post-deactivation access.

The loader can transport handles as ordinary ABI values, but this repository
does not claim an MLPL native-handle value until upstream exposes that value
contract. The current `sw-mlpl` static registry enables colon-qualified scalar
invocation; facade imports, compiler parity, dynamic loading, arrays, and
native-handle values remain separate upstream capabilities.
