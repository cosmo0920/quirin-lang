#!/usr/bin/env bash
# The singleplex pilot after W45-FB-RETRY-LIFETIME's merge: `quirin-audit
# compare` between the merged-slice runs at 864c3d97 and 046accf6, the per-
# contract summary, and the verdict comparison with the retry card's archived
# slice at 046accf6. Run from the repository root.
# usage: singleplex.sh BINARY
set -u
binary="$1"
private=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-contract-revision-mode
out=docs/reports/w45-contract-revision-mode/singleplex
mkdir -p "$out"
"$binary" compare \
  --before "$private/rev-864c3d97/merged-slice/resource-diagnostics.json" \
  --after "$private/rev-046accf6/merged-slice/resource-diagnostics.json" \
  --output "$out/compare-revisions.json" > "$out/compare-revisions.console" 2>&1
echo "compare: $?"
python3 docs/reports/w45-contract-revision-mode/scripts/singleplex_summary.py > "$out/summary.json"
echo "summary: $?"
python3 docs/reports/w45-contract-revision-mode/scripts/slice_verdicts.py \
  "$private/rev-046accf6/merged-slice/resource-diagnostics.json" \
  docs/reports/w45-fb-retry-lifetime/artifacts/slice-after-046accf6/resource-diagnostics.json.gz \
  > "$out/slice-verdicts-vs-retry-card.json"
echo "verdicts: $?"
