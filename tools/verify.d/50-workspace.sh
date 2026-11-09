#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
command -v node >/dev/null || { echo "50-workspace: node is not on PATH" >&2; exit 1; }
major=$(node -p 'process.versions.node.split(".")[0]')
[ "$major" -ge 24 ] || { echo "50-workspace: node $(node --version) is older than 24" >&2; exit 1; }
npm --version >/dev/null || { echo "50-workspace: npm does not answer" >&2; exit 1; }
phase() {
  local start=$SECONDS
  "${@:2}" || return
  local line="time: 50-workspace $1 $((SECONDS - start)) s"
  if [ -n "${NG_GATE_TIMES:-}" ]; then echo "$line" >>"$NG_GATE_TIMES"; else echo "$line"; fi
}
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
cd clinical-workspace
phase install npm ci --ignore-scripts --no-audit --no-fund
phase typecheck npm run typecheck
phase test npm test
phase build env WORKSPACE_OUT_DIR="$out/build" npm run build
