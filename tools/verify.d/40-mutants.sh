#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
table=tools/gate/mutants.toml
python3 -I tools/mutate.py "$table" --check
case "${NG_GATE_MODE:-full}:${NG_GATE_SCOPE:-everything}:${NG_GATE_BASE:--}" in
  changed:*) ;;
  merge:everything:* | merge:*:- | full:*) python3 -I tools/mutate.py "$table" ;;
  *) python3 -I tools/mutate.py "$table" --since "$NG_GATE_BASE" ;;
esac
