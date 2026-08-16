set shell := ["sh", "-cu"]

# Show available repository tasks.
default:
    @just --list

# Run all native mlplunit tests; arguments select paths, tags, or filters.
tests *args:
    ./scripts/run-tests {{args}}

# Run Rust tests across the extension workspace.
rust-tests *args:
    ./scripts/run-rust-tests {{args}}

# List tests discovered by mlplunit without executing them.
list-tests:
    ./scripts/run-tests --list

# Print selected tools without installing or replacing them.
mlpl-path:
    ./scripts/select-mlpl

mlplunit-path:
    ./scripts/select-mlplunit

# Open the interactive MLPL-driven native cube window.
cube-3d:
    ./scripts/run-3d-cube

# Run the complete pre-commit gate.
check:
    ./scripts/check
