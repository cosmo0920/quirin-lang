#!/usr/bin/env bash
# finalize.sh -- card W47-FLB-LIB-LIFECYCLE: compare AFTER (private/ll/after)
# with BEFORE (private/ll/before) and with the archived baseline on the three
# routes (scripts/identity.py), archive every route whose report differs from
# BEFORE as the new baseline under artifacts/after/, and archive the
# condvar/join census AFTER (private/ll/census-final) with its comparison
# against BEFORE (private/ll/census-before, which reproduces the archived
# census byte for byte).
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
art=docs/reports/w47-flb-lib-lifecycle/artifacts
base=docs/reports/w47-loop-owner-instance-flow/integration/artifacts
archive=docs/reports/w47-summary-validation-protected-formals/scripts/archive.py
mkdir -p "$art" || exit 2
python3 "$here/identity.py" private/ll/before private/ll/before/slice "$base" > "$art/identity-before.json" || exit 1
python3 "$here/identity.py" private/ll/after private/ll/after/slice "$base" > "$art/identity-after.json" || exit 1
rm -rf "$art/after"
mkdir -p "$art/after"
for route in corpus system slice; do
  if ! cmp -s "private/ll/before/$route/resource-diagnostics.json" "private/ll/after/$route/resource-diagnostics.json"; then
    python3 "$archive" "private/ll/after/$route" "$art/after/$route" || exit 1
  fi
done
cp private/ll/after/binary.sha256 "$art/after/binary.sha256"
cp private/ll/before/binary.sha256 "$art/before-binary.sha256"
rm -rf "$art/census"
python3 "$archive" private/ll/census-final "$art/census" || exit 1
python3 "$here/compare_census.py" private/ll/census-before/fixed.json private/ll/census-final/fixed.json > "$art/compare-fixed.json" || exit 1
python3 "$here/compare_census.py" private/ll/census-before/previous.json private/ll/census-final/previous.json > "$art/compare-previous.json" || exit 1
