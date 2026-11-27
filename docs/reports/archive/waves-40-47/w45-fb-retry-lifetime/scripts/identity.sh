#!/usr/bin/env bash
# Compare one arbiter run with a baseline, with the checkout root aliased.
# usage: identity.sh RUN_DIR BASELINE_DIR OUT_PREFIX
# Writes OUT_PREFIX.json (the comparison), OUT_PREFIX.stderr and OUT_PREFIX.exit.
set -u
run="$1"; baseline="$2"; out="$3"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
python3 "$repo/docs/reports/w43-return-expression/scripts/baseline_identity.py" "$run" "$baseline" \
  --root-alias "$repo=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da" \
  > "$out.json" 2> "$out.stderr"
echo $? > "$out.exit"
cat "$out.exit"
