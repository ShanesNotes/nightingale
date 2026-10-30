#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
sketch=docs/architecture/solver-contract-sketch
cd "$sketch"
RUSTFLAGS="-D warnings" cargo check --all-targets --locked
cargo test --doc --locked
python3 -I ../../../tools/gate/gate.py codes "$sketch"
python3 -I evidence/census-check.py
