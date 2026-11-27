#!/usr/bin/env bash
# Compare one corpus-arbiter run with a baseline (W43's baseline_identity.py),
# the run's worktree root aliased to the baseline's.
# usage: identity.sh RUN_DIR BASELINE_DIR OUT_JSON
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
baseline_root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da
python3 "$repo/docs/reports/w43-return-expression/scripts/baseline_identity.py" \
  "$1" "$2" --root-alias "$repo=$baseline_root" > "$3" 2> "$3.err"
code=$?
echo "$code" > "$3.exit"
echo "identity exit $code"
