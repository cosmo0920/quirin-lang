#!/usr/bin/env bash
# summarise.sh BEFORE AFTER [COMPARE_NAME] -- NONNULL-MEMBER-UNIFIED-QUALIFIER:
# the per-route totals (totals.py) of both runs and the row comparison
# (compare.py) into the report's artifacts (compare.json, or
# COMPARE_NAME.json). Nothing piped into a recorded exit.
set -u
before="$1"; after="$2"; name="${3:-compare}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
art="$here/../artifacts"
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
cd "$repo" || exit 2
for run in "$before" "$after"; do
  python3 "$here/totals.py" "private/nmuq/$run/resource-diagnostics.json" "private/nmuq/$run-null.summary.json" "$art/totals-$run-corpus.json" > /dev/null
  python3 "$here/totals.py" "private/nmuq/$run-system/resource-diagnostics.json" "private/nmuq/$run-system-null.summary.json" "$art/totals-$run-system.json" > /dev/null
  python3 "$here/totals.py" "$slice/nmuq-$run/resource-diagnostics.json" "private/nmuq/$run-slice-null.summary.json" "$art/totals-$run-slice.json" > /dev/null
done
python3 - "$art" "$before" "$after" <<'PY'
import json, sys
art, runs = sys.argv[1], sys.argv[2:]
for run in runs:
    for route in ("corpus", "system", "slice"):
        d = json.load(open(f"{art}/totals-{run}-{route}.json"))
        print(run, route, d["report_sha256"][:8], d["custody"], d["null"])
PY
python3 "$here/compare.py" "$before" "$after" "$art/$name.json"
