#!/bin/sh
# Archive the AFTER artifacts and the two diff documents (card AOM-BINDING-POISON).
set -eu
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a8c5f623cda3030bb
OUT="$WT/docs/reports/aom-binding-poison/artifacts"
RUN="$WT/measure/runs/after"
cd "$WT"
mkdir -p "$OUT"
gzip -9 -c "$RUN/resource-diagnostics.json"                 > "$OUT/diagnostics.json.gz"
gzip -9 -c "$RUN/monkey-source.json"                        > "$OUT/source.json.gz"
gzip -9 -c "$RUN/resource-diagnostics.chunk-contracts.json" > "$OUT/chunk-contracts.json.gz"
gzip -9 -c "$RUN/resource-diagnostics.source-effects.json"  > "$OUT/source-effects.json.gz"
gzip -9 -c "$RUN/resource-diagnostics.contracts.json"       > "$OUT/contracts.json.gz"
gzip -9 -c measure/diff-baseline-to-before.json             > "$OUT/diff-baseline-to-before.json.gz"
gzip -9 -c measure/diff-baseline-to-after.json              > "$OUT/diff-baseline-to-after.json.gz"
ls -l "$OUT"
