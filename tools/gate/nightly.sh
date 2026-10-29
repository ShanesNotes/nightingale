#!/usr/bin/env bash
#   tools/gate/nightly.sh <log>   0 green or already green for this commit, verify's status when red,
#                                 2 refused (usage, a dirty tree, or HEAD is not origin/main)
set -euo pipefail
cd "$(dirname "$0")/../.."
[ $# -eq 1 ] || { echo "usage: tools/gate/nightly.sh <log>" >&2; exit 2; }
log=$1
[ -z "$(git status --porcelain)" ] || { echo "nightly: the working tree is dirty; refused"; exit 2; }
head=$(git rev-parse HEAD)
main=$(git rev-parse --verify --quiet refs/remotes/origin/main) || { echo "nightly: no origin/main; refused"; exit 2; }
[ "$head" = "$main" ] || { echo "nightly: HEAD $head is not origin/main $main; refused"; exit 2; }
if python3 -I tools/gate/gate.py nightly-passed "$head"; then
  echo "nightly: $head already passed"
  exit 0
fi
mkdir -p "$(dirname "$log")"
status=0
bash tools/verify --full >"$log" 2>&1 || status=$?
python3 -I tools/gate/gate.py nightly-record "$status" "$log"
exit "$status"
