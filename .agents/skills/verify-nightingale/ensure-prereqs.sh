#!/usr/bin/env bash
# Install what tools/verify needs: the Rust toolchain rust-toolchain.toml pins, with rustfmt and
# clippy, and python3 3.11 or newer. Safe to rerun; it changes nothing when both are present.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
channel=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)
[ -n "$channel" ] || { echo "ensure-prereqs: no channel in rust-toolchain.toml"; exit 1; }
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
rustup toolchain install "$channel" --profile minimal --component rustfmt --component clippy
python3 -I -c 'import sys; sys.exit(0 if sys.version_info >= (3, 11) else 1)' \
  || { echo "ensure-prereqs: python3 3.11 or newer is required"; exit 1; }
echo "ensure-prereqs: rustc $(rustc --version | cut -d' ' -f2), $(python3 --version)"
