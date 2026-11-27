#!/usr/bin/env bash
# finalize.sh -- compare the final AFTER runs (private/cj/final) with BEFORE
# (private/cj/before), archive them as this card's new baseline, archive the
# condvar census, and write the caller census of the AFTER runs.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
art=docs/reports/w47-condvar-join-publication/artifacts
slice_before=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/cj-before
slice_final=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/cj-final
mkdir -p "$art" || exit 2
python3 "$here/compare.py" private/cj/before/corpus private/cj/final/corpus "$art/compare-corpus.json" > /dev/null || exit 1
python3 "$here/compare.py" private/cj/before/system private/cj/final/system "$art/compare-system.json" > /dev/null || exit 1
python3 "$here/compare.py" "$slice_before" "$slice_final" "$art/compare-slice.json" > /dev/null || exit 1
rm -rf "$art/after"
mkdir -p "$art/after"
python3 docs/reports/w47-summary-validation-protected-formals/scripts/archive.py private/cj/final/corpus "$art/after/corpus" || exit 1
python3 docs/reports/w47-summary-validation-protected-formals/scripts/archive.py private/cj/final/system "$art/after/system" || exit 1
python3 docs/reports/w47-summary-validation-protected-formals/scripts/archive.py "$slice_final" "$art/after/slice" || exit 1
cp private/cj/final/binary.sha256 "$art/after/binary.sha256"
cp private/cj/before/binary.sha256 "$art/before-binary.sha256"
rm -rf "$art/census"
python3 docs/reports/w47-summary-validation-protected-formals/scripts/archive.py private/cj/census-final "$art/census" || exit 1
python3 "$here/summary.py" private/cj/census-final/previous.json private/cj/census-final/fixed.json "$art/census-summary.json" > /dev/null || exit 1
rm -rf "$art/callers-before"
cp -r private/cj/callers-before "$art/callers-before"
