#!/bin/sh
# RULE36-PILOT: the identity run and the switch-measured contribution on the
# three live profiles, in one `quirin-audit measure` run: BASE is the
# integrator's published, tool-verified baseline of the card's base
# (`baseline:<sha>`, built with QCORE_AUDIT_BUILD_COMMIT=<sha>), TIP the card's
# code tip (`rev:`), built by the tool at its fixed path with the same explicit
# build commit label so the report artifacts can be byte-compared, checked
# against PREDICTION.md (`--declared`: no row moves), plus `tip-off`, the same
# TIP binary under QCORE_GATEWAY_RULES_OFF=36 (rule 36's contribution,
# tip-off -> tip). Wall-time quota 24 h (GATEWAY-RULE-API hit the 10 h one).
# Usage: identity.sh WORKTREE OUT_DIR BASE_SHA TIP_REV   (OUT_DIR fresh, outside the worktree)
set -u
W=$1
OUT=$2
BASE=$3
TIP=$4
cd "$W" || exit 2
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.start"
"$W/target/release/quirin-audit" --plugin-path "$W/target/release" --output "$OUT" \
  --timeout-ms 86400000 measure \
  --profile eight-home --profile fluent-bit-system --profile fluent-bit-tail-pause \
  --side base=baseline:"$BASE" --side tip=rev:"$TIP" --rules-off 36 \
  --declared docs/reports/rule36-pilot/PREDICTION.md \
  --build-commit "$BASE" \
  --baselines /media/Data/GitHub/quirin-lang/private/baselines \
  --inputs-base /media/Data/GitHub/quirin-lang --repository . >"$OUT.log" 2>&1
echo $? >"$OUT.exit"
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.end"
