#!/bin/sh
# Archive the AFTER artifacts and the three diff documents
# (card W4-RECURSIVE-SUMMARIES, wave 30).
set -eu
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a34fba7055c858fd7
OUT="$WT/docs/reports/w4-recursive-summaries/artifacts"
RUN="$WT/measure/runs/after"
cd "$WT"
mkdir -p "$OUT"

# The BEFORE set itself is not archived: this card changes no analysis
# input, so its five artifacts are byte-identical to the AFTER set below
# (measured, logs/byte-identity-before-to-after.txt), and its diagnostics
# and chunk-contracts are byte-identical to
# docs/reports/w29-corpus-contracts/artifacts, which is not touched
# (standing constraint 21).
gzip -9 -c "$RUN/resource-diagnostics.json"                 > "$OUT/diagnostics.json.gz"
gzip -9 -c "$RUN/monkey-source.json"                        > "$OUT/source.json.gz"
gzip -9 -c "$RUN/resource-diagnostics.chunk-contracts.json" > "$OUT/chunk-contracts.json.gz"
gzip -9 -c "$RUN/resource-diagnostics.source-effects.json"  > "$OUT/source-effects.json.gz"
gzip -9 -c "$RUN/resource-diagnostics.contracts.json"       > "$OUT/contracts.json.gz"
gzip -9 -c measure/diff-baseline-to-before.json             > "$OUT/diff-baseline-to-before.json.gz"
gzip -9 -c measure/diff-baseline-to-after.json              > "$OUT/diff-baseline-to-after.json.gz"
gzip -9 -c measure/diff-before-to-after.json                > "$OUT/diff-before-to-after.json.gz"
ls -l "$OUT"
echo ARCHIVE_OK
