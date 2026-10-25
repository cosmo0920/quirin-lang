#!/bin/sh
# CFG-RELEASE comparisons, per route: bytes of every published artifact
# (AUDIT-CANONICAL-PATHS compare.py, no substitution), verdict-level equality
# (verdicts.py) and the pre-pass summary keys (time_key.py), BASE vs TIP.
# Usage: compare_all.sh WORKTREE RUNS_DIR OUT_DIR
set -u
W=$1
R=$2
OUT=$3
S=$W/docs/reports/audit-canonical-paths/scripts
mkdir -p "$OUT"
for route in corpus slice system; do
  python3 "$S/compare.py" "$route" "$R/base/$route" "$R/tip/$route" "$OUT/$route-bytes.json"
  echo "$route bytes exit $?"
  if [ "$route" = corpus ]; then
    report=corpus/resource-diagnostics.json
    contracts=corpus/resource-diagnostics.contracts.json
  else
    report=resource-diagnostics.json
    contracts=resource-diagnostics.contracts.json
  fi
  python3 "$S/verdicts.py" "$R/base/$route/$report" "$R/tip/$route/$report" \
    "$R/base/$route/$route-null.jsonl" "$R/tip/$route/$route-null.jsonl" >"$OUT/$route-verdicts.json"
  echo "$route verdicts exit $?"
  python3 "$S/time_key.py" "$R/base/$route/$contracts" "$R/tip/$route/$contracts" >"$OUT/$route-time-key.json"
  echo "$route time-key exit $?"
done
