# Extension Packages

An extension package keeps its public MLPL facade beside a platform-indexed
native artifact manifest:

```text
hello/
├── extension.toml
├── module.mlpl
└── native/
    ├── aarch64-apple-darwin/libmlpl_extension_hello.dylib
    └── x86_64-unknown-linux-gnu/libmlpl_extension_hello.so
```

`extensions/hello/extension.toml` is the source manifest. Native artifacts are
build outputs and remain ignored; a packaging workflow copies the selected
artifact into the declared location rather than committing local binaries.

## Resolution contract

The loader accepts an explicit manifest path and exact Rust target triple. It
does not search the current directory or guess a compatible architecture. The
manifest declares the public name, semantic version, ABI version, MLPL facade,
private native namespace, and one artifact path per target.

Resolution rejects:

- unsupported ABI versions or target triples;
- duplicate target entries or duplicate public package names;
- absolute, parent, current-directory, or otherwise non-normal path components;
- missing modules and native artifacts;
- canonical paths that escape the package root through a symlink;
- a loaded native descriptor whose namespace differs from the manifest.

Available platform names are sorted before diagnostics, making errors stable
regardless of manifest order. Metadata and canonical paths are owned by the
resolved package.

## Namespace boundary

`hello` is the public MLPL package. `_hello` is its private native namespace.
The Rust library registers `_hello.answer`, `_hello.fail`, and `_hello.panic`;
`module.mlpl` owns public composition and presentation. A structural gate
prevents MLPL tests from calling the private seam directly.

Until sw-MLPL gains the host import hook recorded in `upstream-contract.md`,
the facade accepts a typed native value explicitly in its test. This proves
ordinary MLPL composition and namespace discipline without misrepresenting a
downstream harness as language integration.

## Native3d package

`extensions/native3d/extension.toml` and `module.mlpl` apply the same package
layout to the renderer-neutral `_native3d` provider. Its V1 C descriptor exposes
bulk line and filled-box replacement, perspective/orthographic camera state,
stable-ID box picking and selection, deterministic headless frame state, and
typed viewer teardown. Both SWTOS and MLOS may call this same API after their
MLPL adapters map producer data to generic arrays.

Until sw-MLPL accepts native handles as user-function parameters, handle-taking
calls use `_native3d:*` directly after loading. The helper module can wrap
creation and pure projection-name validation, but does not pretend it can wrap
the remaining handle calls. This limitation is tracked in
[`upstream-contract.md`](upstream-contract.md#extension-handles-through-user-functions).

The packaged cdylib does not open a window. A stock CLI can load and exercise
its headless state API, but winit requires a main-thread event loop and ongoing
host event delivery that the callback-free V1 extension contract does not yet
provide. The custom native3d host remains the honest interactive window path.
