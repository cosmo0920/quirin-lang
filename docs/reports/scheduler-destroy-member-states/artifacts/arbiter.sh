#!/bin/sh
# arbiter.sh OUT [ROUTES]: quirin-audit measure --declared, base export
# (89fa31d21) against tip export, on the LERI card's preparations
# (docs/reports/scheduler-destroy-member-states/artifacts/inputs.json).
R=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ad32a2c41eddd6a5e
I=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ad857989f04ad049c
cd "$R" || exit 2
OUT=$1
ROUTES=${2:-corpus,system,slice}
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh \
  "$R/target/release/quirin-audit" --plugin-path "$R/target/release" --output "$OUT" \
  --timeout-ms 36000000 measure \
  --inputs "$R/docs/reports/scheduler-destroy-member-states/artifacts/inputs.json" \
  --inputs-base "$I" --repository "$R" --routes "$ROUTES" --no-slots \
  --side "base=$R/private/base-src" --side "tip=$R/private/tip-src" \
  --declared "$R/docs/reports/scheduler-destroy-member-states/PREDICTION.md" \
  > "$OUT.log" 2>&1
echo $? > "$OUT.exit"
cat "$OUT.exit"
