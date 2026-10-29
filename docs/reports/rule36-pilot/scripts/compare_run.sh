#!/bin/sh
# RULE36-PILOT: the identity comparison alone, BASE (the published baseline)
# against an already measured TIP side (`run:<dir>`, read as it is; nothing is
# re-run), checked against PREDICTION.md. Used when a full run's switched side
# failed for an environmental reason after its TIP side completed.
# Usage: compare_run.sh WORKTREE OUT_DIR BASE_SHA TIP_SIDE_DIR
set -u
W=$1
OUT=$2
BASE=$3
SIDE=$4
cd "$W" || exit 2
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.start"
"$W/target/release/quirin-audit" --plugin-path "$W/target/release" --output "$OUT" \
  --timeout-ms 86400000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json \
  --side base=baseline:"$BASE" --side tip=run:"$SIDE" \
  --declared docs/reports/rule36-pilot/PREDICTION.md \
  --baselines /media/Data/GitHub/quirin-lang/private/baselines \
  --inputs-base /media/Data/GitHub/quirin-lang --repository . >"$OUT.log" 2>&1
echo $? >"$OUT.exit"
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.end"
