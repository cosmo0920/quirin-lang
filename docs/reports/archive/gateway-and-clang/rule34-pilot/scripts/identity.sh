#!/bin/sh
# RULE34-PILOT: the identity run and the switch-measured contribution on the
# three live profiles, in one `quirin-audit measure` run: BASE (the
# integrator's published `baseline:80c8aea98`, every file digest verified by
# the tool) against TIP (`rev:` the card's code commit, built by the tool at
# its fixed path), checked against PREDICTION.md (`--declared`: no row
# moves), plus `tip-off`, the same TIP binary under QCORE_GATEWAY_RULES_OFF=34
# (rule 34's contribution, tip-off -> tip). The wall-time quota is 24 h
# because earlier pilots' runs took 10-14 h under slot contention.
# Usage: identity.sh WORKTREE OUT_DIR TIP_REV   (OUT_DIR fresh)
set -u
W=$1
OUT=$2
TIP=$3
cd "$W" || exit 2
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.start"
"$W/target/release/quirin-audit" --plugin-path "$W/target/release" --output "$OUT" \
  --timeout-ms 86400000 measure \
  --profile eight-home --profile fluent-bit-system --profile fluent-bit-tail-pause \
  --side base=baseline:80c8aea989029c10b3dfbd0179715b7ea050262c \
  --side tip=rev:"$TIP" --rules-off 34 \
  --declared docs/reports/rule34-pilot/PREDICTION.md \
  --baselines /media/Data/GitHub/quirin-lang/private/baselines \
  --inputs-base /media/Data/GitHub/quirin-lang --repository . >"$OUT.log" 2>&1
echo $? >"$OUT.exit"
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.end"
