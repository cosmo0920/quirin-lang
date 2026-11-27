#!/usr/bin/env bash
# compare-all.sh BEFORE AFTER -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: the
# artifacts of one BEFORE/AFTER pair: summary.json (summarize.py),
# standing.json (standing.py), compare-{corpus,system,slice}.json
# (compare.py) and null-compare.txt (null_compare.py). Exits to
# private/mpuh/compare-*.exit.
set -u
before="$1"; after="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/mpuh"
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
art="$here/../artifacts"
python3 "$here/summarize.py" "$before" "$after" > "$out/compare-summary.log" 2>&1
echo $? > "$out/compare-summary.exit"
python3 "$here/standing.py" "$before" "$after" > "$out/compare-standing.log" 2>&1
echo $? > "$out/compare-standing.exit"
python3 "$here/compare.py" "$out/$before" "$out/$after" "$art/compare-corpus.json" > "$out/compare-corpus.log" 2>&1
echo $? > "$out/compare-corpus.exit"
python3 "$here/compare.py" "$out/$before-system" "$out/$after-system" "$art/compare-system.json" > "$out/compare-system.log" 2>&1
echo $? > "$out/compare-system.exit"
python3 "$here/compare.py" "$slice/mpuh-$before" "$slice/mpuh-$after" "$art/compare-slice.json" > "$out/compare-slice.log" 2>&1
echo $? > "$out/compare-slice.exit"
python3 "$here/null_compare.py" "$before-null" "$after-null" "$before-system-null" "$after-system-null" \
  "$before-slice-null" "$after-slice-null" > "$art/null-compare.txt" 2> "$out/compare-null.log"
echo $? > "$out/compare-null.exit"
