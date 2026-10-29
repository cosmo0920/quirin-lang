#!/bin/sh
# RULE35-PILOT: the identity run and the switch-measured contribution on the
# three live profiles, in one `quirin-audit measure` run: BASE (`rev:` the
# card's base) against TIP (`rev:` the card's tip), both built by the tool at
# its fixed path with one explicit build commit so their report artifacts can
# be byte-compared, checked against PREDICTION.md (`--declared`: no row moves),
# plus `tip-off`, the same TIP binary under QCORE_GATEWAY_RULES_OFF=35 (rule
# 35's contribution, tip-off -> tip). The wall-time quota is raised to 24 h
# because GATEWAY-RULE-API's run hit the 10 h quota under slot contention.
# Usage: identity.sh WORKTREE OUT_DIR BASE_REV TIP_REV   (OUT_DIR fresh, outside the worktree)
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
  --side base=rev:"$BASE" --side tip=rev:"$TIP" --rules-off 35 \
  --declared docs/reports/rule35-pilot/PREDICTION.md \
  --build-commit rule35-pilot-identity \
  --inputs-base /media/Data/GitHub/quirin-lang --repository . >"$OUT.log" 2>&1
echo $? >"$OUT.exit"
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.end"
