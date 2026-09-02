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

# Open the playable MLPL-owned native tic-tac-toe window.
tic-tac-toe:
    ./scripts/run-tic-tac-toe

# Open the editable MLPL-owned native Life plane.
life-3d:
    ./scripts/run-life-3d

# Open wrapped Life on an MLPL-owned native 3D torus.
life-torus:
    ./scripts/run-life-torus

# Measure bounded Model Atlas scanning against growing sparse model files.
model-atlas-memory-evidence:
    ./scripts/run-model-atlas-memory-evidence

# Validate and summarize the cross-repository Model Atlas interchange.
model-atlas-contract:
    ./scripts/run-model-atlas-contract

# Open the interactive native 3D tensor city.
model-atlas:
    ./scripts/run-model-atlas

# Choose and inspect a real local Safetensors model without reading its payload.
model-atlas-file:
    ./scripts/run-model-atlas-file

# Open a bounded, read-only native disk-usage snapshot.
disk-usage:
    ./scripts/run-disk-usage

# Pick a confined MP3/Ogg file and visualize bounded stereo spectrum chunks.
audio-spectrum:
    ./scripts/run-audio-spectrum

# Explore bounded samples from real Safetensors or GGUF weight tensors.
weight-distribution:
    ./scripts/run-weight-distribution

# Validate the offline-first Rust/Yew ML microscope.
microscope-web-check:
    ./scripts/check-microscope-web

# Serve the microscope viewer on its documented loopback URL.
microscope-web:
    ./scripts/run-microscope-web

# Build pinned static microscope assets into the ignored dist directory.
microscope-web-build:
    ./scripts/build-microscope-web

# Open the opt-in native GPU point-scene smoke fixture.
point-cloud-smoke:
    ./scripts/run-point-cloud-smoke

# Open the deterministic MLPL-owned native point-cloud teaching app.
point-cloud:
    ./scripts/run-point-cloud

# Report bounded release-mode CPU evidence for the generic point path.
point-cloud-acceptance:
    ./scripts/check-point-cloud-acceptance

# Run the complete pre-commit gate.
check:
    ./scripts/check
