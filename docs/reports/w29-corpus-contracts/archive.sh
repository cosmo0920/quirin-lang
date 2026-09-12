#!/bin/sh
# Archive the AFTER artifacts and the three diff documents
# (card W29-CORPUS-CONTRACTS, wave 29).
set -eu
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a44c40b2d06b9644e
OUT="$WT/docs/reports/w29-corpus-contracts/artifacts"
RUN="$WT/measure/runs/after2"
cd "$WT"
mkdir -p "$OUT"

# The BEFORE set itself is not archived: its `diagnostics` and
# `chunk-contracts` are byte-identical to
# `docs/reports/gateway-c-location/artifacts`, which is not touched
# (standing constraint 21). The gzipped BEFORE set the itemizer compared
# against was produced by measure/gzip_before.py.
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
