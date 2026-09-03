# Capturing graphical demos

Browser and native demos require different capture tools. `playwright-cli`
can navigate and screenshot the Rust/Yew microscope because it renders in a
browser. It cannot inspect a native `winit`/`wgpu` window such as Life torus.

On macOS, `scripts/capture-native-demo` samples a fixed screen rectangle with
the system `screencapture` command. It uses `cwebp` for a representative still
and `ffmpeg` plus `img2webp` for a downscaled animation. Temporary PNG frames
live under the system temporary directory and are deleted; only compact WebP
outputs are written to `captures/`.

## Life torus

Start the demo and choose a visible preset:

```sh
just life-torus
# Focus the window, press G for a glider, then Space to run.
```

Determine the logical-pixel rectangle around the window with macOS Screenshot
or a full-screen image. Retina screenshots contain twice as many output pixels
as the `X,Y,WIDTH,HEIGHT` rectangle accepted by `screencapture`. Then run:

```sh
CAPTURE_RECT=415,122,900,732 just capture-life-torus
```

The command waits five seconds so the demo can be brought to the foreground,
captures 16 frames at 500 ms intervals, scales them to 720 pixels wide, and
writes `captures/life-torus.webp` plus
`captures/life-torus-animated.webp`. Pass explicit arguments to change the
defaults:

```sh
scripts/capture-native-demo life-torus 415,122,900,732 24 250
```

Screen Recording permission is required. Accessibility permission is optional
and only needed for external automation that moves or focuses windows. OBS is
also optional: it is useful for narrated or manually orbited recordings, after
which `ffmpeg` can crop, scale, and convert the recording. The scripted route is
smaller and more repeatable for README loops.

## Other native demos

The capture implementation is generic. Start a demo in one terminal, arrange
and interact with its window, then use the same rectangle from another terminal:

```sh
CAPTURE_RECT=415,122,900,732 just capture-native audio-spectrum 20 350
CAPTURE_RECT=415,122,900,732 just capture-native point-cloud 16 500
CAPTURE_RECT=415,122,900,732 just capture-native model-atlas 12 650
```

Capturing every native demo is unnecessary and would make the repository and
README noisy. Prefer a small gallery where each asset demonstrates a different
capability:

1. `audio-spectrum` is the best next animation: choose an MP3 or Ogg file and
   capture changing spectrum chunks while playback runs.
2. `point-cloud` demonstrates bulk points, camera motion, stable-ID selection,
   and retained updates.
3. `model-atlas` or `weight-distribution` demonstrates bounded real-data
   inspection; capture only fixtures or data whose redistribution is allowed.
4. `tic-tac-toe` and `life-3d` are useful stills, but add less than the torus
   animation unless documenting a particular interaction.

Use a descriptive output name, keep the 720-pixel default, and add an asset to
the README only when it communicates behavior not already visible there.

## Browser captures

Use the installed `playwright-cli` rather than the similarly named
`playwright` command. A typical static capture flow is:

```sh
playwright-cli open http://127.0.0.1:8080
playwright-cli resize 1280 800
playwright-cli screenshot
playwright-cli close
```

The CLI stores its screenshot in its session output directory; copy only the
final compressed asset into `captures/`. Browser-to-server microscope execution
remains distinct from offline fixture playback and must not be implied by a
static screenshot.
