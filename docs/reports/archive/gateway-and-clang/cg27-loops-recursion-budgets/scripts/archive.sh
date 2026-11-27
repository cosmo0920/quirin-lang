#!/usr/bin/env bash
# archive.sh -- CG27-LOOPS-RECURSION-BUDGETS: write the BEFORE/AFTER
# comparisons, censuses and digests of the three routes into artifacts/.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
art="$here/../artifacts"
private="$repo/private/cg27lrb"
slices=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
mkdir -p "$art" || exit 2
python3 "$here/compare.py" "$private/before" "$private/after" --json "$art/corpus-comparison.json" > /dev/null || exit 1
python3 "$here/compare.py" "$private/before-system" "$private/after-system" --json "$art/system-comparison.json" > /dev/null || exit 1
python3 "$here/compare.py" "$slices/cg27lrb-before" "$slices/cg27lrb-after" --json "$art/slice-comparison.json" > /dev/null || exit 1
python3 "$here/census.py" "$private/before" --json "$art/census-corpus.json" > /dev/null || exit 1
python3 "$here/census.py" "$private/before-system" --json "$art/census-system.json" > /dev/null || exit 1
python3 "$here/census.py" "$slices/cg27lrb-before" --json "$art/census-slice.json" > /dev/null || exit 1
python3 "$here/census.py" "$private/after" --json "$art/census-corpus-after.json" > /dev/null || exit 1
{
  sha256sum "$private/quirin-audit-before" "$private/quirin-audit-after"
  for run in before after before-system after-system; do
    sha256sum "$private/$run/resource-diagnostics.json"
  done
  sha256sum "$slices/cg27lrb-before/resource-diagnostics.json" "$slices/cg27lrb-after/resource-diagnostics.json"
} | sed "s#$repo/##; s#$slices/##" > "$art/digests.txt"
python3 "$here/parity_diff.py" "$private/before-parity.jsonl" "$private/after-parity.jsonl" > "$art/corpus-parity-diff.txt" || exit 1
