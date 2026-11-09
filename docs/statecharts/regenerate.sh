#!/usr/bin/env bash
# Checks every statechart with TLC and redraws its Mermaid chart: system, engine and documentation here, and
# the step statechart through check-step.sh. Each <name>.tla is a model, its view names the states and arrows,
# and <name>.mmd is the chart TLC's state graph gives. The engine statechart is drawn small through a VIEW and
# checked again larger, with no VIEW, a checkpoint and an export taken any number of times and the certificate
# run. Needs Node 22.18 or later and the pstack statechart skill's scripts/statechart command, which runs TLC
# (installed with mise on its first run). REPORTS, when set, keeps each TLC report there:
#   STATECHART=<skill dir>/scripts/statechart bash docs/statecharts/regenerate.sh
set -euo pipefail
cd "$(dirname "$0")"
: "${STATECHART:?set STATECHART to scripts/statechart in the statechart skill}"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

check() {
  local name=$1 model=$2 cfg=$3 view=$4
  if "$STATECHART" "$model.tla" --config "$cfg" --view "$view" --out "$work/$name" > /dev/null; then
    status=holds
  else
    status=breaks
  fi
  if [[ -n ${REPORTS:-} ]]; then
    mkdir -p "$REPORTS"
    cp "$work/$name/report.md" "$REPORTS/$name.md"
  fi
}

for chart in system:system-view.ts documentation:documentation-view.ts engine:view.ts; do
  name=${chart%%:*}
  check "$name" "$name" "$name.cfg" "${chart#*:}"
  [[ $status == holds ]] || { echo "$name: a rule breaks" >&2; exit 1; }
  cp "$work/$name/chart.mmd" "$name.mmd"
  echo "$name: every rule holds"
done

sed -e 's/MaxRevision = 1/MaxRevision = 2/; s/Coverage = 1/Coverage = 2/; s/Branches = {1}/Branches = {1, 2}/' \
  -e 's/Certifies = FALSE/Certifies = TRUE/; s/Overwrites = FALSE/Overwrites = TRUE/; /^VIEW /d' engine.cfg > "$work/engine-check.cfg"
check engine-check engine "$work/engine-check.cfg" view.ts
[[ $status == holds ]] || { echo "engine-check: a rule breaks" >&2; exit 1; }
echo "engine-check: every rule holds"

bash check-step.sh
