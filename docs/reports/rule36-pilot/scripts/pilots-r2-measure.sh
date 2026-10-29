#!/bin/sh
# Pilot round 2 (brief pilots-r2-evidence.md): the one shared measurement for
# RULE33-PILOT, RULE34-PILOT and RULE36-PILOT. BASE is main without the three
# moves (rev:0d15e7b19), TIP main with them (rev:62c8e461c), both built by the
# tool at its fixed path under one explicit build-commit label so their report
# artifacts can be byte-compared; checked against the combined prediction
# (`--declared`: no row moves); plus `tip-off` (33, 34, 36 off together) and
# `tip-off-<id>` (each alone), from the same TIP binary, for the per-rule
# contribution ledgers and their interactions. Three live profiles; 24 h quota.
# Usage: pilots-r2-measure.sh WORKTREE OUT_DIR   (OUT_DIR fresh)
set -u
W=$1
OUT=$2
cd "$W" || exit 2
df -h /media/Data >"$OUT.df-start"
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.start"
"$W/target/release/quirin-audit" --plugin-path "$W/target/release" --output "$OUT" \
  --timeout-ms 86400000 measure \
  --profile eight-home --profile fluent-bit-system --profile fluent-bit-tail-pause \
  --side base=rev:0d15e7b19abaddd6fffd2f676df749949ef8c679 \
  --side tip=rev:62c8e461cf00628ab6c26a45dcdb243e91d79924 \
  --rules-off 33,34,36 --per-rule \
  --declared docs/reports/rule36-pilot/PREDICTION-r2.md \
  --build-commit pilots-r2-identity \
  --inputs-base /media/Data/GitHub/quirin-lang --repository . >"$OUT.log" 2>&1
echo $? >"$OUT.exit"
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.end"
df -h /media/Data >"$OUT.df-end"
