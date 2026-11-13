#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
# 10-rust's build settings, so the tests it passed stand for the rest part's baselines.
export RUSTFLAGS="" RUSTDOCFLAGS=""
unset CARGO_ENCODED_RUSTFLAGS CARGO_ENCODED_RUSTDOCFLAGS CLIPPY_CONF_DIR RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER RUSTC_BOOTSTRAP \
  UPDATE_EXPECT
exec python3 -I tools/gate/gate.py mutants "$@"
