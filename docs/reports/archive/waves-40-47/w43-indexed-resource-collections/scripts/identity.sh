#!/bin/bash
# Compare one audit run with the W43-INDIRECT-ACCESS-GENERAL baseline under
# this worktree's checkout-root alias. Usage: identity.sh RUN_DIR [BASELINE_DIR]
ROOT=$(cd "$(dirname "$0")/../../../.." && pwd)
BASELINE=${2:-$ROOT/docs/reports/w43-indirect-access-general/artifacts}
CANONICAL=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a8d24f7f717a987c1
python3 "$ROOT/docs/reports/w43-return-expression/scripts/baseline_identity.py" "$1" "$BASELINE" \
  --root-alias "$ROOT=$CANONICAL"
