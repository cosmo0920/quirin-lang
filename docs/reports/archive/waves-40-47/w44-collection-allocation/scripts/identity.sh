#!/bin/bash
# Compare one audit run with the W43-INDEXED-RESOURCE-COLLECTIONS integration
# baseline, aliasing this worktree's root to the integrator worktree's.
# Usage: identity.sh RUN_DIR OUT_JSON [BASELINE_DIR]
ROOT=$(cd "$(dirname "$0")/../../../.." && pwd)
BASELINE=${3:-$ROOT/docs/reports/w43-indexed-resource-collections/integration/artifacts}
CANONICAL=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da
python3 "$ROOT/docs/reports/w43-return-expression/scripts/baseline_identity.py" "$1" "$BASELINE" \
  --root-alias "$ROOT=$CANONICAL" > "$2"
echo $?
