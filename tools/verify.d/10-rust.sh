#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
# Cargo takes the first flag source it finds, so an empty RUSTFLAGS shuts out a host config's
# `--cap-lints allow`, which would silently disable forbid. RUSTC_BOOTSTRAP would admit #![feature] and
# with it the unstable float methods the bans probe does not classify; UPDATE_EXPECT would rewrite a trace.
export RUSTFLAGS="" RUSTDOCFLAGS=""
unset CARGO_ENCODED_RUSTFLAGS CARGO_ENCODED_RUSTDOCFLAGS CLIPPY_CONF_DIR RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER RUSTC_BOOTSTRAP \
  UPDATE_EXPECT
if [ -z "${NG_GATE_CRATES+set}" ]; then
  plan=$(python3 -I tools/gate/gate.py plan full) || { printf '%s\n' "$plan"; exit 1; }
  NG_GATE_CRATES=$(sed -n 's/^crates=//p' <<<"$plan")
fi
cargo fmt --all --check
pkgs=()
for crate in $NG_GATE_CRATES; do
  pkgs+=(-p "$crate")
done
if [ ${#pkgs[@]} -gt 0 ]; then
  cargo clippy --locked --all-targets "${pkgs[@]}" -- -D warnings
  if [ "${NG_GATE_MODE:-}" = full ]; then
    cargo test --locked "${pkgs[@]}" -- --include-ignored
  else
    cargo test --locked "${pkgs[@]}"
  fi
fi
for smoke in ${NG_GATE_SMOKE:-}; do
  case " $NG_GATE_CRATES " in
    *" ${smoke%%:*} "*) ;;
    *) cargo test --locked -p "${smoke%%:*}" --test "${smoke#*:}" ;;
  esac
done
python3 -I tools/gate/gate.py doctests
[ "${NG_GATE_MODE:-}" = changed ] || python3 -I tools/gate/gate.py bans
