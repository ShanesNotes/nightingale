#!/usr/bin/env bash
# Checks the step model. Every rule holds for the design on main, and each past step design breaks the rule
# its history records. Redraws step.mmd from the small run. Needs the pstack statechart skill's
# scripts/statechart command, which runs TLC (installed with mise on its first run):
#   STATECHART=<skill dir>/scripts/statechart bash docs/statecharts/check-step.sh
set -euo pipefail
cd "$(dirname "$0")"
: "${STATECHART:?set STATECHART to scripts/statechart in the statechart skill}"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

config() {
  local name=$1 line
  shift
  cp step.cfg "$work/$name.cfg"
  for line in "$@"; do
    sed -i "s|^  ${line%% *} .*|  $line|" "$work/$name.cfg"
  done
}

holds() {
  "$STATECHART" step.tla --config "$work/$1.cfg" --view step-view.ts --out "$work/$1" > /dev/null
  echo "$1: every rule holds"
}

breaks() {
  local design=$1 rule=$2
  shift 2
  config "$design" "Design = \"$design\"" "$@"
  if "$STATECHART" step.tla --config "$work/$design.cfg" --out "$work/$design" > /dev/null; then
    echo "$design: every rule held, but $rule should break" >&2
    exit 1
  fi
  if ! grep -q "^## $rule breaks" "$work/$design/report.md"; then
    echo "$design: $rule did not break" >&2
    exit 1
  fi
  echo "$design: $rule breaks"
}

branches=('Span = 1' 'Ids = {"b"}' 'Moves = {}' 'Slots = {1, 2}' 'Cores <- PassingCore' 'Guards = {0}')

config main
config branches "${branches[@]}"
config chart 'Ids = {"a"}' 'Moves = {"a"}' 'Slots = {}' 'Cores <- ChartCores' 'Guards = {0, 1}'
holds main
holds branches
holds chart
cp "$work/chart/chart.mmd" step.mmd

breaks clipAtIssue LateInputGivesKnownBits 'Guards = {0}'
breaks clipFirst BackDatedInputRefused 'Guards = {0}'
breaks noFence LateInputGivesKnownBits 'Guards = {0}'
breaks cutThenLand LateInputGivesKnownBits
breaks fireAfterCut LateInputGivesKnownBits
breaks ledgerRoot RewindLandsOnThisBranch "${branches[@]}"
breaks rollback PublishedStepsStay 'Guards = {0}'
