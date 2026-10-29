#!/usr/bin/env bash
# Install what tools/verify needs: the Rust toolchain rust-toolchain.toml pins, with rustfmt and
# clippy, and python3 3.11 or newer; then turn on the pre-push hook. Safe to rerun; it changes nothing
# when all three are in place.
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
# core.hooksPath lives in the clone's shared config, so it reaches every worktree. The path is relative:
# git resolves it in the worktree where the hook runs, so a branch without tools/gate/hooks runs no hook.
want=tools/gate/hooks
have=$(git config --get core.hooksPath || true)
hooks=$(git rev-parse --git-common-dir)/hooks
active=$(find "$hooks" -maxdepth 1 -type f ! -name '*.sample' 2>/dev/null | head -n 1 || true)
if [ "$have" = "$want" ]; then
  echo "ensure-prereqs: the pre-push hook is on (core.hooksPath is $want)"
elif [ -n "$have" ]; then
  echo "ensure-prereqs: core.hooksPath is $have; left alone, so pushes do not run tools/verify --changed"
elif [ -n "$active" ]; then
  echo "ensure-prereqs: $active is an active hook; left alone, so pushes do not run tools/verify --changed"
else
  git config core.hooksPath "$want"
  echo "ensure-prereqs: the pre-push hook is on for every worktree of this clone (undo: git config --unset core.hooksPath)"
fi
