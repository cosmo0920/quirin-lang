#!/bin/sh
# evidence.sh SIDE... : CG10-SMALL-BUNDLE: the census and named-row evidence of
# each measured side (base, tip) from this worktree's private/sb/run-SIDE.
W=$(cd "$(dirname "$0")/../../../.." && pwd)
R=$W/docs/reports/cg10-small-bundle
P=/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
mkdir -p "$R/census" "$R/evidence"
for side in "$@"; do
  J=$W/private/sb/run-$side/resource-diagnostics.json
  python3 "$R/scripts/totals.py" "$J" > "$R/evidence/totals-$side.json"
  python3 "$R/scripts/fnptr_formals.py" "$J" "$P" > "$R/census/fnptr-formals-$side.jsonl"
  python3 "$R/scripts/callers.py" "$R/census/fnptr-formals-$side.jsonl" "$J" > "$R/census/callers-$side.txt"
done
if [ -f "$W/private/sb/run-tip/resource-diagnostics.json" ]; then
  B=$W/private/sb/run-base/resource-diagnostics.json
  T=$W/private/sb/run-tip/resource-diagnostics.json
  python3 "$R/scripts/compare.py" "$B" "$T" "$R/evidence/compare-base-tip.json"
  python3 "$R/scripts/changed_rows.py" "$B" "$T" > "$R/evidence/changed-rows.txt"
  python3 "$R/scripts/named_rows.py" "$B" "$T" > "$R/evidence/named-rows.jsonl"
fi
