# Rust/Yew ML microscope viewer

Run `just microscope-web`, then open <http://127.0.0.1:8080>. The page embeds
the revision- and hash-pinned MM01 matrix, LR01 linear-regression, and KM01
K-means phase recordings, so Previous, Next, Play/Pause, observation selection,
and seek work without a network service. LR01 deliberately proves that
retained-index navigation moves from producer step 2 directly to step 4.

The viewer is a generic numeric consumer. Rust does not contain matrix,
regression, K-means, optimizer, or other lesson semantics. It validates the version-zero
recording before retention, groups slash-separated opaque names for display,
chooses a presentation from tensor rank, and always renders exact tabular values
alongside graphics. Playback reviews retained history; it never pauses or
reverses the MLPL evaluator.

## Architecture and safety

`mlpl-microscope-model` has no DOM, Yew, network, or evaluator dependency. It
owns ordered validation, the pure reducer, finite summaries, exact fallbacks,
and repeated-name series. `mlpl-microscope-web` owns incremental UTF-8-safe SSE
framing, bounded ordered live assembly, keyboard controls, motion preference,
and escaped Yew DOM construction. Producer text is never trusted as HTML, and
producer SVG or JavaScript is not accepted.

The vendored inputs under `integration/ml-microscope` are pinned to
`demo-ml-microscope` revision
`ff15ec7da2f9055983aa72c43dfe01da92a2d4aa`. The producer's fixture index
retains its own source revision `9ea15cf6ece58752238f2fee799b4ce008a4aa8c`;
the later vendored revision adds the KM01 handoff and documentation without
changing those indexed bytes. `SHA256SUMS` covers the index, schema, recordings,
and lesson sources. Builds and tests never read an adjacent mutable checkout.

KM01 confirms that the existing generic renderer handles `[6,2]` point and
squared-distance matrices, repeated assignment/update observation names,
alternating slash-separated phase prefixes, and a final zero centroid delta.
All remain ordinary shape-directed observations: there is no `KMeansViewer` or
algorithm-specific Rust rendering path.

## Checks and release build

Run `just microscope-web-check` for native model/adversarial/reducer tests, SSE
byte-split and terminal-flow tests, a WASM target check, provenance verification,
reduced-motion and exact-fallback assertions, and `sw-checklist` over both Rust
crates. Run `just microscope-web-build` for a locked release build in the ignored
`dist/microscope-web` directory. The same static output serves on macOS or Linux.

The UI defaults to paused. With `prefers-reduced-motion: reduce`, CSS transition
and animation durations become zero and playback still requires an explicit
Play action. Keyboard Left/Right and Home/End operate outside editor/form focus;
buttons and the seek input retain native keyboard semantics.

## Live execution boundary

The checked transport accepts `ready`, ordered `frame`, `done`, and `error` SSE
events, preserves call order for observations sharing a step, and rejects
malformed, non-finite, out-of-order, or over-budget input without exposing a
partial recording. A local server can be started with:

```sh
../sw-mlpl/target/release/mlpl-serve \
  --bind 127.0.0.1:6464 --auth disabled \
  --cors-allow http://127.0.0.1:8080
```

The current page intentionally labels its Run control as requiring that server;
offline playback is the accepted deployment mode. Browser-side session POST and
stream wiring are the next live acceptance layer and are not claimed here.
