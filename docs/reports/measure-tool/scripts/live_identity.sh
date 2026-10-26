#!/bin/sh
# MEASURE-TOOL: the tool's own identity run on the configured live routes
# (system selection, tail-pause slice): BASE 373e41ee8 against the card's
# commit TIP_REV (no analyzer change), both built by the tool from their
# revisions and run at the fixed path. Then BASE is published as a baseline
# and reused, after identity verification, against TIP's recorded run.
# BASE_REV (environment, default 373e41ee8) names the base revision.
# Usage: live_identity.sh WORKTREE OUT_DIR TIP_REV [BUILD_COMMIT]   (OUT_DIR fresh, outside the worktree)
# BUILD_COMMIT: one explicit QCORE_AUDIT_BUILD_COMMIT for both sides, so the
# evidence versions agree and the report artifacts can be byte-compared.
set -u
W=$1
OUT=$2
TIP=$3
BC=${4:+--build-commit $4}
BASE=${BASE_REV:-373e41ee8}
cd "$W" || exit 2
H="$W/target/debug/quirin-audit --plugin-path $W/target/debug"
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.start"
$H --output "$OUT/measure" --timeout-ms 28800000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json --routes system,slice \
  --side base=rev:"$BASE" --side tip=rev:"$TIP" $BC \
  --inputs-base /media/Data/GitHub/quirin-lang --repository . >"$OUT.measure.log" 2>&1
echo $? >"$OUT.measure.exit"
FULL=$(git rev-parse "$BASE")
$H --output "$OUT/baselines/$FULL" measure publish --from "$OUT/measure/sides/base" \
  >"$OUT.publish.log" 2>&1
echo $? >"$OUT.publish.exit"
$H --output "$OUT/reuse" --timeout-ms 3600000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json --routes system,slice \
  --side base=baseline:"$BASE" --side tip=run:"$OUT/measure/sides/tip" \
  --baselines "$OUT/baselines" --inputs-base /media/Data/GitHub/quirin-lang --repository . \
  >"$OUT.reuse.log" 2>&1
echo $? >"$OUT.reuse.exit"
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.end"
