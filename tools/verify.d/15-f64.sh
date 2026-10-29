#!/usr/bin/env bash
# No bare f64 or f32 in the public API of the plan's crates unless the crate's paths.toml row allows that
# item, read from rustdoc JSON; then, outside a changed run, the probe crate proves the scan still bites.
set -euo pipefail
cd "$(dirname "$0")/../.."
export RUSTFLAGS="" RUSTDOCFLAGS=""
unset CARGO_ENCODED_RUSTFLAGS CARGO_ENCODED_RUSTDOCFLAGS CLIPPY_CONF_DIR RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER
python3 -I tools/gate/gate.py f64
