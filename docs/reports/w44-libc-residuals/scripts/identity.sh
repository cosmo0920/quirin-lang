#!/bin/sh
# Compare one run with an archived baseline, aliasing this worktree's root to
# the integrator worktree root the baseline was captured in.
# Usage: identity.sh RUN_DIR BASELINE_ARTIFACT_DIR OUT_TXT
ROOT=$(cd "$(dirname "$0")/../../../.." && pwd)
CANONICAL=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da
python3 "$ROOT/docs/reports/w43-return-expression/scripts/baseline_identity.py" "$1" "$2" --root-alias "$ROOT=$CANONICAL" > "$3" 2>&1
code=$?
echo "exit=$code" >> "$3"
exit $code
