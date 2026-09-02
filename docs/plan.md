# Rust Extensions and Native Visualization Plan

## Outcome

Demonstrate that an independently built Rust extension can add substantial
native capability to sw-MLPL without adding domain concepts to the language.
The same MLPL-facing module must work from the REPL, interpreted scripts, and
compiled programs. The flagship application is an interactive embedding
explorer, but the first proof is deliberately a headless `hello` extension so
ABI and lifecycle failures are separable from graphics failures.

This repository owns examples, acceptance tests, packaging examples, and any
temporary public SDK crates needed to prove the design. General runtime and
language changes belong in `../sw-mlpl` under separately authorized work.

## Delivery snapshot

As of 2026-09-02, the extension foundation, safe SDK, native line renderer,
pointer/camera contract, tic-tac-toe, Life plane and torus, Model Atlas,
disk-usage explorer, audio spectrum player, weight-distribution explorer, Yew
microscope viewer, and repository-wide retained-scene migration are delivered.
The native point-cloud saga has delivered its renderer-neutral bulk-array
contract, deterministic headless projection/raster slice, and native wgpu/winit
point pipeline. The numbered
sections below preserve the original architectural sequence rather than serving
as the executable queue. Embedding/PCA remains a future candidate after point
rendering, interaction, and acceptance.

## Recommended architecture

The public programming model is an MLPL module, not an `ffi.call` API:

```text
MLPL module facade
    -> extension registry and package resolver
        -> dynamic loader or statically registered provider
            -> versioned C ABI
                -> safe Rust SDK and macros
                    -> extension implementation
```

The ABI is intentionally narrow: scalars, UTF-8 strings, bytes, dense numeric
array views, errors, function/type metadata, host callbacks, and opaque typed
handles. It does not expose evaluator internals or Rust types. A `cdylib`
exports one versioned entry point. The loader retains the library for every
registered function and live native handle, validates all descriptor lengths
and pointers before registration, contains panics, and deactivates before any
attempt to unload.

Rust exposes small mechanical primitives under a private native namespace.
An MLPL `module.mlpl` supplies defaults and composition under the public
namespace. Both dynamic and statically linked registration implement the same
registry contract so application source does not change at packaging time.

Native 3D uses `wgpu` and `winit` on Metal (macOS) and Vulkan or another native
wgpu backend (Linux). V1 starts with blocking, script-friendly display. A live
REPL viewer follows only after the host event-loop contract is explicit.

## Non-negotiable contracts

- ABI compatibility is checked before any function is callable.
- Unsafe code is isolated behind safe SDK and loader APIs.
- Dense arrays carry dtype, rank, shape, strides, mutability, ownership, and
  call-lifetime semantics. Zero-copy is a measured optimization, not a V1
  promise.
- Native resources use extension-scoped, type-tagged generational IDs. No raw
  pointer is ever an MLPL value.
- Function metadata includes names, documentation, arguments, defaults, return
  types, handle types, and array shape constraints for help and diagnostics.
- Deactivation rejects new calls. True unloading requires zero active calls,
  handles, callbacks, extension threads, and host services; otherwise the
  library remains resident with an actionable diagnostic.
- Graphics and UI primitives are domain-neutral. Embedding search, PCA,
  clustering, filtering, quiz behavior, and presentation choices stay in MLPL.
- Dynamic loading never searches the current directory implicitly. Manifests,
  canonical paths, platform triples, checksums, and explicit development paths
  make resolution deterministic.

## Testing and repository workflow

All executable work is TDD. Rust starts with the smallest failing unit or
integration test and is verified with scoped `cargo test` commands. MLPL uses
the sibling-repository convention: root `mlplunit.conf`, `tests/test_*.mlpl`,
native `@test` declarations, shared `u:assert_*` helpers, and
`u:run_registered_tests()`. Scripts select `$MLPL` and `$MLPLUNIT` first, then
PATH, then documented adjacent builds; they never install binaries.

A thin root `justfile` will delegate `tests`, `rust-tests`, `demos`, `audit`,
and `check` to portable scripts. `just check` becomes the pre-commit gate once
the scaffold exists. Headless contract tests are mandatory. Window/GPU smoke
tests are opt-in and platform-labelled so CI or remote agents do not mistake a
missing display for an ABI failure.

Every implementation session follows AgentRail: `next`, `begin`, red/green
work, scoped gates, named-file commit, `complete`, stop. Append-only AgentRail
state is changed only through its CLI.

Every step also has the same release gate: run scoped `cargo test` and
mlplunit as applicable plus the full pre-commit check; update affected docs;
audit `.gitignore`; use `git status --short` to confirm all intended source,
test, fixture, documentation, configuration, and AgentRail files are tracked;
stage only named files; make a detailed commit; complete the AgentRail step;
commit completion metadata if it changed; and successfully run
`git push origin main`. Work stays on `main`; feature branches, PRs, `gh`, and
GitHub Actions are outside this repository's workflow.

## Delivery order and capability gates

### 1. Extension foundation

Build the smallest external `hello` extension and prove descriptor validation,
function registration, invocation, errors, deactivation, manifest resolution,
and dynamic/shared-library packaging. Add MLPL-facing tests only when the
upstream host exposes the required public registration path. This slice proves
the ecosystem boundary without graphics dependencies.

Gate: one independently built library is discoverable, rejects ABI mismatch,
registers a namespaced function, returns a typed result/error, and cannot be
called after deactivation. Static-provider parity is designed and tested at
the registry layer.

### 2. Safe SDK, metadata, arrays, and handles

First factor the stabilized registration path behind equivalent dynamic and
static provider guards, proving the same hello contract through each. Then add
safe conversions and ergonomic macros only after hand-written ABI calls
stabilize. Introduce read-only dense array views and typed generational handles
with explicit ownership. Test invalid UTF-8, null/overflowing descriptors,
wrong dtype/rank/shape, strided input policy, stale handles, wrong-extension
handles, panics, concurrent calls, and shutdown ordering.

Gate: an extension author implements a typed function without writing unsafe
code; MLPL help can describe it; a bulk `[N,3]` input crosses in one call; all
negative contract tests fail closed.

### 3. Native wireframe renderer fixture

Build the smallest visible vertical slice first: MLPL generates bulk `[8,3]`
cube positions and `[12,2]` edges, while a renderer-neutral Rust crate validates
the generic line-scene contract. Add deterministic headless projection and draw
planning before an opt-in wgpu/winit window. The native API reports generic
events; MLPL maps keyboard input to width, height, length, signed rotation
speed, RGBA presets, and line thickness and submits bulk scene updates.

Until upstream arrays and handles reach third-party providers, use deterministic
JSON as an explicit file bridge. Do not describe that bridge as final extension
integration or zero-copy transport.

Gate: MLPL and Rust contract tests agree on shapes, bounds, and serialization;
headless renderer tests are deterministic; macOS and Linux compile native
wgpu/winit code; and an opt-in smoke run displays the rotating cube without a
browser stack. A Rust-owned cube-specific control loop does not satisfy the
interactive acceptance gate.

### 4. Native point-cloud vertical slice

Implement only a viewer, orbit/zoom camera, and point cloud with scalar or
per-point size/color/id attributes. Begin with a deterministic generated array
and a blocking `show` path. Keep renderer state behind typed handles and keep
GPU/window code out of the loader.

Gate: macOS and Linux builds compile through native wgpu/winit backends;
headless geometry/camera/upload tests pass; an opt-in native smoke run displays
a real window with no browser stack.

### 5. Live viewer, updates, and picking

Define the host event-loop service before adding `open`. Add bulk position and
attribute updates, close/deactivate behavior, resize/input events, and either a
host-provided pick ray or stable point IDs. Start with polling or bounded event
queues; callbacks/event streams wait until reentrancy and lifetime rules are
tested.

Gate: a viewer survives multiple REPL evaluations, bulk updates reuse the
object safely, picking maps to an MLPL row, and close/deactivation leaves no
callable stale handles or background work.

### 6. Embedding and PCA explorer

Use a tiny checked-in or deterministically generated embedding matrix. Keep
centering, PCA/projection, cluster/color/size arrays, masks, selection, and
metadata lookup in MLPL. Compare a pure MLPL reference with an accelerated
extension only if both use the same fixtures and tolerances.

Gate: one MLPL program computes or loads `[N,D]`, renders `[N,3]`, colors and
sizes points with parallel arrays, selects a row, and explains every array
shape. The demo runs as script and from the REPL; compiled parity is exercised
when the upstream compiler supports extension packaging.

### 7. RAG semantic-space explorer

Add portable chunk metadata and embeddings, vectorized cosine similarity,
top-k selection, query-driven size/highlight arrays, and a native text/details
panel. Avoid an external vector database initially; later backends can compare
brute-force MLPL, a SIMD Rust extension, and a real index through one contract.

Gate: deterministic queries return golden ranked rows, the same row IDs drive
visual selection and metadata, and unsupported/malformed datasets fail without
partial native state.

### 8. Broader visualization demonstrations

Add world capitals, then star catalog, only after the point/line/event APIs are
stable. Capitals introduces geographic transforms and ragged line strips;
stars introduces larger true-3D data. Higher-level scatter, vector field,
surface, and visualization helpers remain MLPL wrappers over a small native
primitive set.

Gate: no domain knowledge enters Rust, and each new demo reuses the public
third-party extension path without privileged runtime hooks.

## Original demo order and remaining direction

1. `hello` — delivered.
2. `wireframe-cube` — delivered, including retained updates.
3. `point-cloud` — active; contract, headless, and native GPU slices are delivered.
4. `live-point-cloud` — future persistent-handle work after the point slice.
5. `embedding-pca` — future candidate after point rendering/picking.
6. `rag-explorer` — future candidate.
7. `world-capitals` — future candidate.
8. `star-explorer` — future candidate.

UMAP and t-SNE follow PCA, not precede it: PCA is deterministic, easier to
test, and exercises the core array algebra without adding stochastic optimizer
and dependency questions. The spinning cube remains a renderer fixture and
teaching bridge rather than the headline application.

## Deferred decisions

- True `dlclose` and hot reload remain deferred until quiescence is proven.
- Zero-copy, mutable borrowed arrays, callbacks, asynchronous event streams,
  shader APIs, textures, and arbitrary native UI widgets are not V1 promises.
- A separate reusable native3d package may be split out after this repository
  proves the public authoring experience.
- Package signing, trust policy, distribution registry, and dependency
  sandboxing require a security-focused saga after local manifests work.
