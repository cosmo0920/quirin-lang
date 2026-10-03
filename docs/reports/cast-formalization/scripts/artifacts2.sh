#!/usr/bin/env bash
# artifacts2.sh -- CAST-FORMALIZATION: the pair of record after the rebase
# onto main 9c5f2079c (`before2`, `after2`, scripts/remeasure.sh) into
# artifacts/rebased/; the first pair (base cb3085365) stays in artifacts/.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
p="$repo/private/castf"
a="$repo/docs/reports/cast-formalization/artifacts/rebased"
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
mkdir -p "$a" || exit 2
for f in census-before2-corpus.json census-before2-corpus.txt census-after2-corpus.json \
  census-after2-corpus.txt compare2-corpus.json compare2-system.json compare2-slice.json \
  families-before2.json families-after2.json cast-refusals-before2.json cast-refusals-after2.json; do
  cp "$p/$f" "$a/$f" || exit 1
done
cp "$p/tip-probe/out-after2/rows.txt" "$a/tip-probe-rows-after2.txt" || exit 1
python3 "$here/merged_totals.py" "$p/before2" "$p/after2" "$p/before2-system" "$p/after2-system" \
  "$slice/castf-before2" "$slice/castf-after2" > "$a/totals.txt" || exit 1
cat "$p/before2.binary.sha256" "$p/after2.binary.sha256" >> "$a/totals.txt"
