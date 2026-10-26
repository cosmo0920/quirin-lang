#!/bin/sh
# GATEWAY-RULE-API: the identity run on the three live profiles (BASE against
# the card's TIP, both built by the tool from their revisions at the fixed
# path with one explicit build commit, so their report artifacts can be
# byte-compared), with rule 35 switched off as the third side (`tip-off`, the
# same TIP binary under QCORE_GATEWAY_RULES_OFF=35): the real --rules-off run.
# Usage: identity.sh WORKTREE OUT_DIR BASE_REV TIP_REV   (OUT_DIR fresh, outside the worktree)
set -u
W=$1
OUT=$2
BASE=$3
TIP=$4
cd "$W" || exit 2
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.start"
"$W/target/release/quirin-audit" --plugin-path "$W/target/release" --output "$OUT" \
  --timeout-ms 36000000 measure \
  --profile eight-home --profile fluent-bit-system --profile fluent-bit-tail-pause \
  --side base=rev:"$BASE" --side tip=rev:"$TIP" --rules-off 35 \
  --build-commit gateway-rule-api-identity \
  --inputs-base /media/Data/GitHub/quirin-lang --repository . >"$OUT.log" 2>&1
echo $? >"$OUT.exit"
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.end"
