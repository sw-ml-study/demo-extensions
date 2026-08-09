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
