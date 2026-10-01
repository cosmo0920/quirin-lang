#!/usr/bin/env bash
# archive.sh -- CG20-NUMERIC-DOMAIN: archive the BEFORE/AFTER comparisons
# (scripts/measure.sh before|after) and the scheduler property summary into
# artifacts/. Every exit is written to a file; nothing is piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
private="$repo/private/cg20"
slices=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
art="$here/../artifacts"
mkdir -p "$art" || exit 2
python3 "$here/compare.py" "$private/before" "$private/after" \
  --json "$art/corpus-comparison.json" > "$private/compare-corpus.txt" 2>&1
echo $? > "$private/compare-corpus.exit"
python3 "$here/compare.py" "$private/before-system" "$private/after-system" \
  --json "$art/system-comparison.json" > "$private/compare-system.txt" 2>&1
echo $? > "$private/compare-system.exit"
python3 "$here/compare.py" "$slices/cg20-before" "$slices/cg20-after" \
  --json "$art/slice-comparison.json" > "$private/compare-slice.txt" 2>&1
echo $? > "$private/compare-slice.exit"
python3 "$here/parity_identity.py" "$private/before-parity.jsonl" "$private/after-parity.jsonl" \
  > "$art/parity-corpus.txt" 2>&1
echo $? > "$private/parity-corpus.exit"
cp "$private/numeric/summary.txt" "$art/scheduler-numeric.txt"
echo $? > "$private/archive-summary.exit"
