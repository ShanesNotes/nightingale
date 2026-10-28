#!/usr/bin/env bash
# Run full tools/verify from the repo root and print a proof block.
# Exit 0 only when the block says full_proof: yes.
# A non-zero tools/verify status is preserved when that status is already non-zero.
# Exit 2 means tools/verify exited 0 but the run is still not full proof
# (dirty tree, head moved, or the OK line is absent).
set -euo pipefail

die() { echo "prove: $*" >&2; exit 1; }

ROOT=$(git rev-parse --show-toplevel 2>/dev/null) || die "not inside a git checkout"
cd "$ROOT"

ENV_FILE="${NIGHTINGALE_TOOLCHAIN_ENV:-$HOME/.local/nightingale-toolchain/env.sh}"
if [ -f "$ENV_FILE" ]; then
  # shellcheck disable=SC1090
  . "$ENV_FILE"
fi
hash -r

export CARGO_TERM_COLOR=never

start_sha=$(git rev-parse HEAD)
start_dirty=$(git status --porcelain)
start_s=$(date +%s)
start_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)

log=${1:-"/tmp/nightingale-verify-${start_sha}.log"}
mkdir -p "$(dirname "$log")"

set +e
bash tools/verify >"$log" 2>&1
verify_exit=$?
set -e

end_s=$(date +%s)
end_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)
wall=$((end_s - start_s))
end_sha=$(git rev-parse HEAD)
end_dirty=$(git status --porcelain)
log_sha=$(sha256sum "$log" | awk '{print $1}')
final=$(awk 'NF {line=$0} END {print line}' "$log")

if [ -n "$start_dirty" ] || [ -n "$end_dirty" ]; then
  worktree=dirty
else
  worktree=clean
fi
if [ "$start_sha" = "$end_sha" ]; then
  head_unchanged=yes
else
  head_unchanged=no
fi
if [ -n "${CARGO_TARGET_DIR:-}" ]; then
  cargo_target="$CARGO_TARGET_DIR"
else
  cargo_target=default
fi

reason=""
full=yes
if [ "$verify_exit" -ne 0 ]; then
  full=no
  reason="tools/verify exited ${verify_exit}"
elif [ "$final" != "verify: OK" ]; then
  full=no
  reason="final line is '${final}', not 'verify: OK'"
elif [ "$worktree" != clean ]; then
  full=no
  reason="worktree dirty; the commit SHA is not the tree that ran"
elif [ "$head_unchanged" != yes ]; then
  full=no
  reason="HEAD changed during the run (${start_sha} -> ${end_sha})"
else
  reason="exit 0 and final line verify: OK"
fi

proof="${log}.proof"
{
  echo "### Verify proof"
  echo
  echo "- commit: ${start_sha}"
  echo "- command: bash tools/verify"
  echo "- verify_exit: ${verify_exit}"
  echo "- final_line: ${final}"
  echo "- wall_time_s: ${wall}"
  echo "- started_utc: ${start_utc}"
  echo "- finished_utc: ${end_utc}"
  echo "- log: ${log}"
  echo "- log_sha256: ${log_sha}"
  echo "- worktree: ${worktree}"
  echo "- head_unchanged: ${head_unchanged}"
  echo "- cargo_target_dir: ${cargo_target}"
  echo "- full_proof: ${full}"
  echo "- reason: ${reason}"
  echo
  echo '```'
  tail -n 40 "$log"
  echo '```'
} | tee "$proof"

if [ "$full" = yes ]; then
  exit 0
fi
if [ "$verify_exit" -ne 0 ]; then
  exit "$verify_exit"
fi
exit 2
