#!/bin/sh
# Verdict-level equality for one route over the attempt-2 sides.
# Usage: verdicts_all.sh WORKTREE ROUTE
W=$1
ROUTE=$2
R=$W/private/acp/runs2
OUT=$W/docs/reports/audit-canonical-paths/identity
V=$W/docs/reports/audit-canonical-paths/scripts/verdicts.py
if [ "$ROUTE" = corpus ]; then report=corpus/resource-diagnostics.json; else report=resource-diagnostics.json; fi
for side in tip-mirror tip-original base-namespace; do
  python3 "$V" "$R/base-mirror/$ROUTE/$report" "$R/$side/$ROUTE/$report" \
    "$R/base-mirror/$ROUTE/$ROUTE-null.jsonl" "$R/$side/$ROUTE/$ROUTE-null.jsonl" >"$OUT/$ROUTE-verdicts-base-mirror-vs-$side.json"
  echo "$ROUTE base-mirror vs $side: exit $?"
done
