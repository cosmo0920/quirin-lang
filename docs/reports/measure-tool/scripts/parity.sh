#!/bin/sh
# MEASURE-TOOL parity: replay rule 33's DETACH comparison (pinned corpus,
# BASE e282aaccc, TIP 9ecc941d5) through `quirin-audit measure`, both sides
# built by the tool from their revisions and run at one fixed path, the
# pinned inputs re-spelled into the mirror (both revisions predate
# AUDIT-CANONICAL-PATHS) and their pre-composition reports read with
# --legacy-rows. The declared moves are docs/reports/measure-tool/parity/PREDICTION.md.
# Usage: parity.sh WORKTREE OUT_DIR   (OUT_DIR fresh, outside the worktree)
set -u
W=$1
OUT=$2
cd "$W" || exit 2
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.start"
"$W/target/debug/quirin-audit" --plugin-path "$W/target/debug" --output "$OUT" \
  --timeout-ms 28800000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json --routes corpus \
  --side base=rev:e282aaccc --side tip=rev:9ecc941d5 \
  --declared docs/reports/measure-tool/parity/PREDICTION.md --legacy-rows \
  --inputs-base /media/Data/GitHub/quirin-lang --repository . >"$OUT.console.log" 2>&1
echo $? >"$OUT.exit"
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.end"
