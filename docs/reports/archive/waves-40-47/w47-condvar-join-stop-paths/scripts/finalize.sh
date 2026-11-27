#!/usr/bin/env bash
# finalize.sh -- compare AFTER (private/sp/after) with BEFORE (private/sp/before)
# on the three routes, archive the routes that moved (system and slice; the
# corpus is byte-identical and stays the W47-CONDVAR-JOIN-PUBLICATION
# integration baseline), and archive the condvar/join census AFTER
# (measure/census-after2) with its comparison against BEFORE
# (measure/census-before).
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
art=docs/reports/w47-condvar-join-stop-paths/artifacts
slice_before=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/sp-before
slice_after=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/sp-after
compare=docs/reports/w47-condvar-join-publication/scripts/compare.py
archive=docs/reports/w47-summary-validation-protected-formals/scripts/archive.py
mkdir -p "$art" || exit 2
python3 "$compare" private/sp/before/corpus private/sp/after/corpus "$art/compare-corpus.json" > /dev/null || exit 1
python3 "$compare" private/sp/before/system private/sp/after/system "$art/compare-system.json" > /dev/null || exit 1
python3 "$compare" "$slice_before" "$slice_after" "$art/compare-slice.json" > /dev/null || exit 1
cmp private/sp/before/corpus/resource-diagnostics.json private/sp/after/corpus/resource-diagnostics.json || exit 1
rm -rf "$art/after"
mkdir -p "$art/after"
python3 "$archive" private/sp/after/system "$art/after/system" || exit 1
python3 "$archive" "$slice_after" "$art/after/slice" || exit 1
cp private/sp/after/binary.sha256 "$art/after/binary.sha256"
cp private/sp/before/binary.sha256 "$art/before-binary.sha256"
rm -rf "$art/census"
python3 "$archive" measure/census-after2 "$art/census" || exit 1
python3 "$here/compare_census.py" measure/census-before/fixed.json measure/census-after2/fixed.json "$art/compare-fixed.json" > /dev/null || exit 1
python3 "$here/compare_census.py" measure/census-before/previous.json measure/census-after2/previous.json "$art/compare-previous.json" > /dev/null || exit 1
