#!/bin/sh
# RULE32A-PILOT, attempt 2: the identity comparison without re-running either
# side. measure-1 (identity.sh) ran TIP to completion but refused BASE
# `baseline:db2795100`: the baseline was published from `land.py publish`'s
# measurement, whose routes spell the explicit inputs (`inputs/live.json`,
# link map docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json) while
# this card's run spells `--profile` (contract/evaluations/eight-home/
# link-map.json, the same bytes), so the route digests differ. Here BASE is
# that landing measurement's `sides/tip` (rev:db2795100, the directory the
# baseline was published from; landing_equals_baseline.py checks every
# published file's digest against it) and TIP is measure-1's `sides/tip`
# (rev:492f0418a under build-commit db2795100), both read as written
# (`run:`), checked against PREDICTION.md.
# Usage: identity-2.sh WORKTREE OUT_DIR MEASURE1_DIR   (OUT_DIR fresh)
set -u
W=$1
OUT=$2
M1=$3
L=/media/Data/GitHub/quirin-lang/private/measure/landing-db2795100a760efa0a2cbcbe13217bffc3cf4733-20261030T123833260013Z
cd "$W" || exit 2
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.start"
"$W/target/release/quirin-audit" --plugin-path "$W/target/release" --output "$OUT" \
  --timeout-ms 86400000 measure \
  --profile eight-home --profile fluent-bit-system --profile fluent-bit-tail-pause \
  --side base=run:"$L/sides/tip" \
  --side tip=run:"$M1/sides/tip" \
  --declared docs/reports/rule32a-pilot/PREDICTION.md \
  --inputs-base /media/Data/GitHub/quirin-lang --repository . >"$OUT.log" 2>&1
echo $? >"$OUT.exit"
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.end"
