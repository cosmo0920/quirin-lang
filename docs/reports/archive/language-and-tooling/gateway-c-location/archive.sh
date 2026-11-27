#!/bin/sh
# Archive the AFTER artifacts and the three diff documents
# (card GATEWAY-C-LOCATION, wave 27).
set -eu
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a1e06161086be80b0
OUT="$WT/docs/reports/gateway-c-location/artifacts"
RUN="$WT/measure/runs/after"
cd "$WT"
mkdir -p "$OUT" measure/before-gz

# A gzipped BEFORE set, so the standard itemizer can compare BEFORE -> AFTER
# with the same reader it uses for an archive. The BEFORE set itself is not
# archived: its `diagnostics` and `chunk-contracts` are byte-identical to
# `docs/reports/aom-binding-poison/artifacts`, which is not touched
# (standing constraint 21).
gzip -9 -c "$WT/measure/runs/before/resource-diagnostics.json" \
  > measure/before-gz/diagnostics.json.gz
gzip -9 -c "$WT/measure/runs/before/monkey-source.json" \
  > measure/before-gz/source.json.gz
gzip -9 -c "$WT/measure/runs/before/resource-diagnostics.chunk-contracts.json" \
  > measure/before-gz/chunk-contracts.json.gz
gzip -9 -c "$WT/measure/runs/before/resource-diagnostics.source-effects.json" \
  > measure/before-gz/source-effects.json.gz
gzip -9 -c "$WT/measure/runs/before/resource-diagnostics.contracts.json" \
  > measure/before-gz/contracts.json.gz

python3 -B docs/reports/corpus-repin/diff_artifacts.py \
  --archived measure/before-gz --measured "$RUN" \
  --output measure/diffs/diff-before-to-after.json

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
