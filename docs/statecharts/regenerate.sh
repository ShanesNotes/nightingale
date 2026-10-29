#!/usr/bin/env bash
# Redraws the README statechart. engine.tla is the model, view.ts names its states and arrows,
# engine.mmd is the chart TLC's state graph gives, and render.mjs draws engine.svg from it.
# Needs Node 22.18 or later, network for pele@1.9.0, and the pstack statechart skill's
# scripts/statechart command, which runs TLC (installed with mise on its first run):
#   STATECHART=<skill dir>/scripts/statechart docs/statecharts/regenerate.sh
set -euo pipefail
cd "$(dirname "$0")"
: "${STATECHART:?set STATECHART to scripts/statechart in the statechart skill}"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
"$STATECHART" engine.tla --config engine.cfg --view view.ts --out "$work/out"
cp "$work/out/chart.mmd" engine.mmd
npm install --prefix "$work" --no-save --silent pele@1.9.0
cp render.mjs "$work/render.mjs"
node "$work/render.mjs" engine.mmd "$work/out/report.md" engine.svg
