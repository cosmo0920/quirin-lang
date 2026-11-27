#!/usr/bin/env bash
# finalize.sh -- card W47-FLB-LIB-STRICT-PAIRING: compare BEFORE and AFTER
# (private/sp/{before,after}, slices under private/w47-tail/attempt-1/
# strictpair-{before,after}) with the archived baseline on the three routes
# (scripts/identity.py), archive every route whose report differs from
# BEFORE as the new baseline under artifacts/after/, archive the census
# AFTER (private/sp/census-final) with its summary against BEFORE
# (private/sp/census-before, identical to the archived lifecycle census but
# for the module path), and the runtime-test sweep's summary.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
art=docs/reports/w47-flb-lib-strict-pairing/artifacts
base=docs/reports/w47-flb-lib-lifecycle/integration/artifacts
slices=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
archive=docs/reports/w47-summary-validation-protected-formals/scripts/archive.py
mkdir -p "$art" || exit 2
python3 "$here/identity.py" private/sp/before "$slices/strictpair-before" "$base" > "$art/identity-before.json" || exit 1
python3 "$here/identity.py" private/sp/after "$slices/strictpair-after" "$base" > "$art/identity-after.json" || exit 1
rm -rf "$art/after"
mkdir -p "$art/after"
for route in corpus system; do
  if ! cmp -s "private/sp/before/$route/resource-diagnostics.json" "private/sp/after/$route/resource-diagnostics.json"; then
    python3 "$archive" "private/sp/after/$route" "$art/after/$route" || exit 1
  fi
done
if ! cmp -s "$slices/strictpair-before/resource-diagnostics.json" "$slices/strictpair-after/resource-diagnostics.json"; then
  python3 "$archive" "$slices/strictpair-after" "$art/after/slice" || exit 1
fi
cp private/sp/after/binary.sha256 "$art/after/binary.sha256"
cp private/sp/before/binary.sha256 "$art/before-binary.sha256"
rm -rf "$art/census"
python3 "$archive" private/sp/census-final "$art/census" || exit 1
python3 "$here/census_identity.py" private/sp/census-before docs/reports/w47-flb-lib-lifecycle/artifacts/census > "$art/census-before-identity.json" || exit 1
python3 "$here/summarize.py" private/sp/census-final private/sp/census-before > "$art/census-summary.json" || exit 1
mkdir -p "$art/runtime"
python3 "$here/runtime_summary.py" private/sp/runtime-a2 > "$art/runtime/summary.json" || exit 1
cp private/sp/runtime-a2/inputs.sha256 private/sp/runtime-a2/files.txt "$art/runtime/" || exit 1
cp private/sp/runtime-a2/in_ebpf_exec_handler.note "$art/runtime/" || exit 1
for f in private/sp/runtime-a2/*.exit; do
  printf '%s %s\n' "$(basename "$f" .exit)" "$(cat "$f")"
done > "$art/runtime/exits.txt"
