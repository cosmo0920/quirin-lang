#!/bin/sh
# Archive the AFTER artifacts and the three diff documents
# (card W31-CORPUS-CONTRACTS-2, wave 31).
set -eu
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a56d2b9c03bea3365
OUT="$WT/docs/reports/w31-corpus-contracts-2/artifacts"
RUN="$WT/measure/runs/after"
cd "$WT"
mkdir -p "$OUT"

# The BEFORE set itself is not archived: its diagnostics and chunk-contracts
# are byte-identical to docs/reports/w4-recursive-summaries/artifacts, which
# is not touched (standing constraint 21), and the run-to-run identity of the
# other three is recorded in logs/byte-identity-before-to-after.txt.
gzip -9 -c "$RUN/resource-diagnostics.json"                 > "$OUT/diagnostics.json.gz"
gzip -9 -c "$RUN/monkey-source.json"                        > "$OUT/source.json.gz"
gzip -9 -c "$RUN/resource-diagnostics.chunk-contracts.json" > "$OUT/chunk-contracts.json.gz"
gzip -9 -c "$RUN/resource-diagnostics.source-effects.json"  > "$OUT/source-effects.json.gz"
gzip -9 -c "$RUN/resource-diagnostics.contracts.json"       > "$OUT/contracts.json.gz"
gzip -9 -c measure/diffs/diff-baseline-to-before.json       > "$OUT/diff-baseline-to-before.json.gz"
gzip -9 -c measure/diffs/diff-baseline-to-after.json        > "$OUT/diff-baseline-to-after.json.gz"
gzip -9 -c measure/diffs/diff-before-to-after.json          > "$OUT/diff-before-to-after.json.gz"
ls -l "$OUT"
echo ARCHIVE_OK
