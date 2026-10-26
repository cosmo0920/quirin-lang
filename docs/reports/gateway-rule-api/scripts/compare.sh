#!/bin/sh
# GATEWAY-RULE-API: re-compare recorded side directories of an earlier
# measurement (`run:` sides, nothing re-run).
# Usage: compare.sh WORKTREE OUT_DIR SIDE_DIR_BASE SIDE_DIR_TIP   (OUT_DIR fresh)
set -u
W=$1
OUT=$2
cd "$W" || exit 2
"$W/target/release/quirin-audit" --plugin-path "$W/target/release" --output "$OUT" \
  --timeout-ms 3600000 measure \
  --profile eight-home --profile fluent-bit-system --profile fluent-bit-tail-pause \
  --side base=run:"$3" --side tip=run:"$4" \
  --inputs-base /media/Data/GitHub/quirin-lang --repository . >"$OUT.log" 2>&1
echo $? >"$OUT.exit"
