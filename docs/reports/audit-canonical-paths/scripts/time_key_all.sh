#!/bin/sh
# time_key.py over each spelling group of one route's attempt-2 sidecars.
# Usage: time_key_all.sh WORKTREE ROUTE [RUNS_FOR_NAMESPACE]
W=$1
ROUTE=$2
NS_RUNS=${3:-runs2}
A=$W/private/acp
OUT=$W/docs/reports/audit-canonical-paths/identity
T=$W/docs/reports/audit-canonical-paths/scripts/time_key.py
if [ "$ROUTE" = corpus ]; then s=corpus/resource-diagnostics.contracts.json; else s=resource-diagnostics.contracts.json; fi
python3 "$T" "$A/runs2/base-mirror/$ROUTE/$s" "$A/runs2/tip-mirror/$ROUTE/$s" >"$OUT/$ROUTE-time-key-mirror.json"
echo "mirror group: $?"
python3 "$T" "$A/$NS_RUNS/base-namespace/$ROUTE/$s" "$A/runs2/tip-original/$ROUTE/$s" >"$OUT/$ROUTE-time-key-original.json"
echo "original-spelling group: $?"
