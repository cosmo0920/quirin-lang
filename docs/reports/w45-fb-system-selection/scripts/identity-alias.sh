#!/usr/bin/env bash
# Baseline identity of an arbiter run, with the baseline worktree's root spelled
# as an alias of this checkout (a disclosure; the raw comparison is the verdict).
# usage: identity-alias.sh ARBITER_RUN_DIR
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
python3 docs/reports/w43-return-expression/scripts/baseline_identity.py \
  "$1" docs/reports/w45-sub-object-stores/integration/artifacts \
  --root-alias "$repo=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da"
