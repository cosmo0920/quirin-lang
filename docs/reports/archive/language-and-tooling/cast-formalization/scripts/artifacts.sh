#!/usr/bin/env bash
# artifacts.sh -- CAST-FORMALIZATION: copy the measurement summaries from
# private/castf into artifacts/ (the raw reports and parity streams stay
# private).
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
p="$repo/private/castf"
a="$repo/docs/reports/cast-formalization/artifacts"
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
mkdir -p "$a/tip-probe" || exit 2
for f in census-before-corpus.json census-before-corpus.txt census-after-corpus.json \
  census-after-corpus.txt compare-corpus.json compare-system.json compare-slice.json \
  families-before.json families-after.json cast-refusals-before.json cast-refusals-after.json; do
  cp "$p/$f" "$a/$f" || exit 1
done
cp "$p/tip-probe/out-after1/rows.txt" "$a/tip-probe/rows-after.txt" || exit 1
python3 "$here/merged_totals.py" "$p/before" "$p/after1" "$p/before-system" "$p/after1-system" \
  "$slice/castf-before" "$slice/castf-after1" > "$a/totals.txt" || exit 1
cat "$p/before.binary.sha256" "$p/after1.binary.sha256" >> "$a/totals.txt"
