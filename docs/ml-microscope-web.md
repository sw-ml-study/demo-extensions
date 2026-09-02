# Rust/Yew ML microscope viewer

Run `just microscope-web`, then open <http://127.0.0.1:8080>. The page embeds
the revision- and hash-pinned MM01 matrix and LR01 linear-regression recordings,
so Previous, Next, Play/Pause, observation selection, and seek work without a
network service. LR01 deliberately proves that retained-index navigation moves
from producer step 2 directly to step 4.

The viewer is a generic numeric consumer. Rust does not contain matrix,
regression, optimizer, or other lesson semantics. It validates the version-zero
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
`21276a44c29501c37519f5f9f534f54351fbefb4`. `SHA256SUMS` covers the index,
schema, recordings, and lesson sources. Builds and tests never read an adjacent
mutable checkout.

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
