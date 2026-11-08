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
phase() {
  local start=$SECONDS
  "${@:2}" || return
  local line="time: 10-rust $1 $((SECONDS - start)) s"
  if [ -n "${NG_GATE_TIMES:-}" ]; then echo "$line" >>"$NG_GATE_TIMES"; else echo "$line"; fi
}
phase fmt cargo fmt --all --check
pkgs=()
for crate in $NG_GATE_CRATES; do
  pkgs+=(-p "$crate")
done
if [ ${#pkgs[@]} -gt 0 ]; then
  phase clippy cargo clippy --locked --all-targets "${pkgs[@]}" -- -D warnings
  if [ "${NG_GATE_MODE:-}" = full ]; then
    phase test cargo test --locked "${pkgs[@]}" -- --include-ignored
  else
    phase test cargo test --locked "${pkgs[@]}"
  fi
fi
for smoke in ${NG_GATE_SMOKE:-}; do
  case " $NG_GATE_CRATES " in
    *" ${smoke%%:*} "*) ;;
    *) phase "smoke $smoke" cargo test --locked -p "${smoke%%:*}" --test "${smoke#*:}" ;;
  esac
done
phase doctests python3 -I tools/gate/gate.py doctests
phase codes python3 -I tools/gate/gate.py codes
[ "${NG_GATE_MODE:-}" = changed ] || phase bans python3 -I tools/gate/gate.py bans
[ "${NG_GATE_MODE:-}" = changed ] || phase bench cargo check --locked --manifest-path tools/bench/Cargo.toml
# The teacher driver is its own workspace, as the bench is, so the lines above reach neither. Its ignored tests
# admit the arena in their own processes, one at a time.
driver=tools/generative-physiology/driver/Cargo.toml
phase driver-fmt cargo fmt --check --manifest-path "$driver"
[ "${NG_GATE_MODE:-}" = changed ] || phase driver-clippy cargo clippy --locked --all-targets --manifest-path "$driver" \
  -- -D warnings
if [ "${NG_GATE_MODE:-}" = full ]; then
  phase driver-test cargo test --locked --manifest-path "$driver" -- --include-ignored --test-threads=1
elif [ "${NG_GATE_MODE:-}" != changed ]; then
  phase driver-test cargo test --locked --manifest-path "$driver"
fi
[ "${NG_GATE_MODE:-}" = changed ] || phase driver cargo check --locked --manifest-path "$driver"
