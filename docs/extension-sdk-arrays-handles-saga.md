# Extension SDK, Arrays, and Handles

Turn the proven hand-written ABI and dynamic loader into one provider-neutral,
safe authoring contract, then add the bulk numeric arrays and typed native
handles required by native visualization. This remains downstream work; missing
sw-MLPL host hooks stay recorded and require separate authorization.

1. `static-dynamic-provider-parity` — Factor descriptor registration behind retained dynamic and static provider guards, then run the identical hello success, failure, panic, namespace, lifetime, and deactivation contract against both providers.
2. `safe-scalar-conversions` — Add test-first safe SDK conversions for nil, bool, i64, f64, UTF-8, bytes, and owned errors, including malformed tag, reserved-field, lifetime, and boundary cases.
3. `signature-metadata` — Add function/type documentation, arguments, defaults, and return metadata with deterministic validation and help fixtures.
4. `dense-array-views` — Add read-only dense numeric array views with dtype, rank, shape, stride, overflow, alignment, ownership, and call-lifetime tests; prove one bulk `[N,3]` call without claiming zero-copy beyond evidence.
5. `typed-generational-handles` — Add extension-scoped, type-tagged generational handles with stale, wrong-type, wrong-extension, exhaustion, finalization, and deactivation tests.
6. `sdk-macros-acceptance` — Add authoring macros only for stabilized boilerplate, migrate hello to the safe SDK, run dynamic/static parity and MLPL gates, publish acceptance and remaining upstream blockers, close the saga, and stop.

Acceptance: dynamic and static providers share one validated registry path;
ordinary extension authors write no unsafe code; scalar and bulk-array values
fail closed; native resources never cross as raw pointers; generated metadata
matches the hand-written contract; and unavailable sw-MLPL integration is not
misrepresented as complete.
