# Native 3D audio spectrum player

Run `just audio-spectrum`. The launcher prefers the ignored `local-media/`
directory when present and otherwise uses the adjacent `demo-file-processing`
fixtures. Set `AUDIO_SPECTRUM_ROOT` to an absolute directory to choose another
confined root. Discovery is deterministic, skips symlinks, accepts `.mp3` and
`.ogg`, and retains at most 128 relative paths.

The generic Rust boundary uses Symphonia to decode MP3 and Ogg/Vorbis
incrementally. Ogg is a container, not a codec: Opus-in-Ogg is not claimed or
silently treated as Vorbis. Each decoder result sent across the application
boundary contains at most 1,024 stereo PCM frames. No compressed file or full
PCM stream is loaded into memory.

MLPL owns selection, play/pause and seek intent, stereo frequency analysis,
the bass/mid/high mapping, camera behavior, labels, and the mirrored radial
scene. It analyzes a bounded 64-sample window at eight frequencies per channel.
Bass spokes begin nearest the center, higher frequencies fan outward, and
amplitude extends a spoke outward and upward. Blue is bass, green is midrange,
and orange is high frequency.

The renderer retains exactly 16 stable-ID spokes. Every analyzed chunk patches
those 16 objects; it never resends a complete scene. Visualization updates are
single-flight and coalesced: Rust retains only the newest decoded chunk while
MLPL analyzes and acknowledges the current one. Device playback independently
decodes ahead into its bounded queue. Pointer and keyboard events therefore
remain bounded without making audible timing depend on MLPL frame latency.

Audible output uses CPAL's default platform device and requests the source's
native sample rate when the device supports it. The ring buffer retains at most
8,192 stereo frames; the fallback path resamples continuously to the available
device rate. It emits silence on underrun and drops oldest queued samples on
overflow. Decoding ahead independently of visualization acknowledgements, and
preserving one resampling timeline across chunks, removes the chunk-boundary
wow/flutter observed in the first implementation. Space pauses both decode
pacing and the device stream. J/K seeks backward/forward five seconds and
clears pre-seek samples. M returns to the picker. Playback does not write or
alter the source. MP3 and Ogg/Vorbis playback were interactively verified on
the development Mac after this timing correction.

The same source uses CoreAudio on macOS and ALSA on Linux through CPAL. Linux
build hosts need the normal ALSA development package; runtime requires an
available default output device. Headless tests cover decoding, resampling,
buffer bounds, MLPL analysis, retained patches, and applet flow without audio
hardware. An interactive smoke is still required to confirm a particular
device, driver, and desktop session.

The debug measurement decoded the committed fixture's 12,672 stereo frames in
22 bounded chunks in 10.1 ms on the development Mac while asserting a
1,024-frame maximum. This is repeatable fixture evidence, not a universal
throughput guarantee. Structural memory bounds are one
codec packet, one decoder chunk, two 64-sample MLPL analysis windows, 16 scene
objects, and the 8,192-frame playback queue. Run the measurement with:

```sh
cargo test -p mlpl-native3d-window --test audio_decode \
  complete_fixture_decode_never_exceeds_the_chunk_bound -- --nocapture
```

The repo-local `local-media/` directory is deliberately ignored. It permits
testing real personal media without committing it; source, tests, lockfiles,
scripts, and documentation remain tracked.
