#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
want=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)
[ -n "$want" ] || { echo "01-toolchain: no channel in rust-toolchain.toml"; exit 1; }
rustc=$(rustc --version | cut -d' ' -f2) || { echo "01-toolchain: rustc does not run"; exit 1; }
cargo=$(cargo --version | cut -d' ' -f2) || { echo "01-toolchain: cargo does not run"; exit 1; }
[ "$rustc" = "$want" ] || { echo "01-toolchain: rustc $rustc, rust-toolchain.toml pins $want"; exit 1; }
[ "$cargo" = "$want" ] || { echo "01-toolchain: cargo $cargo, rust-toolchain.toml pins $want"; exit 1; }
cargo fmt --version >/dev/null || { echo "01-toolchain: cargo fmt does not answer; add rustfmt"; exit 1; }
cargo clippy --version >/dev/null || { echo "01-toolchain: cargo clippy does not answer; add clippy"; exit 1; }
python3 -I -c 'import sys; sys.exit(0 if sys.version_info >= (3, 11) else 1)' \
  || { echo "01-toolchain: python3 3.11 or newer is required"; exit 1; }
echo "01-toolchain: rustc $rustc, cargo $cargo, $(python3 --version)"
