# Linux tokenizer and verified-download delivery

Validated on 2026-09-22 on Linux x86_64 with rustc 1.88.0. The release
libraries are built locally in the existing manifest package locations:

| Package | Library relative to this repository | SHA-256 of this local build |
|---|---|---|
| `hftok` | `extensions/hftok/native/x86_64-unknown-linux-gnu/libmlpl_extension_hftok.so` | `815f3ec0efac3df883b04c92163681a3ae812190677fca45db598bc1ed85783b` |
| `http` | `extensions/http-client/native/x86_64-unknown-linux-gnu/libmlpl_extension_http_client.so` | `360ce46da300d9ce3bda4db6d2f46c9c737a696cea9df3a1af7f9a99d1164e74` |

These are generated artifacts, ignored by the existing `*.so` rule. Git
publication delivers source and lockfiles; another machine must rebuild or
receive the complete package directories (manifest, facade, native library).
Binary hashes identify this build, not a promise of reproducible binary bytes
across toolchains or source paths. No aarch64 or macOS build was measured here.

## Rebuild and integrate

From this repository on x86_64 Linux:

```sh
cargo build --release --locked -p mlpl-extension-hftok -p mlpl-extension-http-client
mkdir -p extensions/hftok/native/x86_64-unknown-linux-gnu
mkdir -p extensions/http-client/native/x86_64-unknown-linux-gnu
cp target/release/libmlpl_extension_hftok.so extensions/hftok/native/x86_64-unknown-linux-gnu/
cp target/release/libmlpl_extension_http_client.so extensions/http-client/native/x86_64-unknown-linux-gnu/
```

In the consuming MLPL application, load the absolute library paths with
`load_extension`, then include each package's `module.mlpl` facade beneath
the configured include root. Use `u:hftok_load(absolute_tokenizer_path)`,
`u:hftok_encode`, `u:hftok_decode`, and `u:hftok_close`. Download through
`u:http_download(url,expected_bytes,sha256,root,path,chunk_bytes,timeout_ms)`;
`root` is an existing absolute directory, `path` a confined relative filename.
See [tokenizer contract](hftok-extension.md) and
[download contract](network-db-extensions.md).

Use a current MLPL executable with native-handle and array marshaling. This
machine's September 1 installed release executable failed the smoke test;
building from the adjacent `sw-mlpl` commit `6d784660` succeeded without
changing its sources or replacing the installed binary:

```sh
cargo build --locked --manifest-path ../sw-mlpl/components/cli/Cargo.toml \
  --target-dir "$PWD/target/upstream-host" -p mlpl-repl
export MLPL="$PWD/target/upstream-host/debug/mlpl-repl"
```

## Evidence and limits

Focused Rust tests first failed on NFC rejection and separately on added
tokens stored outside `model.vocab`. They now cover canonical composition,
Hangul composition, combining-mark ordering, preservation of compatibility
characters, empty input, null normalization, added-token isolation, unsupported
normalized added tokens, and control-token lookup/decode outside the BPE table.
Existing download tests exercise checksum mismatch, truncation, timeout,
redirect bounds, verified reuse, and cleanup using loopback servers.

Final validation passed: 53 scoped Rust tests and the full `just check` gate
(workspace tests, clippy, formatting and MLPL documentation checks,
upstream-host tests, native mlplunit, and headless integration checks).
The full gate used the fresh `MLPL` executable above and the absolute
`MLPLUNIT` override below. Loopback tests ran with network permission.

The packaged release libraries were loaded by the fresh MLPL executable:

- Real tokenizer: `Qwen/Qwen3-0.6B-Base`, revision
  `da87bfb608c14b7cf20ba1ce41287e8de496c0cd`, `tokenizer.json`.
- Both `Cafe` plus combining acute and `Café` encoded as `[34,2577,963]`;
  decoding returned `Café`. Turn-token input decoded as
  `<|im_start|>Café<|im_end|>`.
- Vocabulary size was 151665; end-of-text, turn-start, and turn-end IDs were
  151643, 151644, and 151645. This Base file lacks think-tag added tokens, so
  their info fields correctly report `-1`.
- `_http:download` fetched 7,031,645 bytes to ignored
  `models/qwen3-tokenizer.json`, verified SHA-256
  `c0382117ea329cdf097041132f6d735924b697924d6f6fc3945713e96ce87539`, and
  reported `reused: false`; a second call reported `reused: true` without a
  transfer. This digest was independently measured from an
  HTTPS fetch at the pinned revision; it is not a publisher-signed digest.

NFC returns normalized text; it cannot promise byte-exact round trips of
decomposed input. Literal added tokens are isolated first. NFC files with
`normalized:true` or unspecified normalized flags on added tokens are refused.
No new chat, prompt, or model-specific semantics run in Rust.

The `../reasoning-from-scratch` checkout is absent here. Its reference suite,
Qwen3 goldens, and 12,000 training prompts were not available. This smoke
evidence is not full reference parity or a throughput acceptance result.
Those were measured later on macOS against the consumer checkout; see
`hftok-acceptance.md`. Integration into that repository remains
its agent's task. No weights were downloaded.

The full gate also needs the adjacent `demo-ml-utils` checkout (used here at
`e6d285d`), `demo-file-processing` audio fixtures (at `71d652e`), and mlplunit.
Local tool override used for mlplunit:
`/disk1/tmp/reasoning-tools/mlplunit/bin/mlplunit`.
