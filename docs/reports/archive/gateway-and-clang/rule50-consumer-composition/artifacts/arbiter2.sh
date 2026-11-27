#!/bin/sh
# arbiter.sh OUT: quirin-audit measure --declared, corpus route, base export
# (0f6e17118, the side arbiter-1 measured) against the second tip export, on this card's preparations
# (docs/reports/rule50-consumer-composition/artifacts/inputs.json).
R=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a4697725f5e3891df
cd "$R" || exit 2
OUT=$1
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh \
  "$R/private/tip2-src/target/release/quirin-audit" --plugin-path "$R/private/tip2-src/target/release" \
  --output "$OUT" --timeout-ms 36000000 measure \
  --inputs "$R/docs/reports/rule50-consumer-composition/artifacts/inputs.json" \
  --inputs-base "$R" --repository "$R" --routes corpus --no-slots \
  --side "base=run:$R/measure/arbiter-1/sides/base" --side "tip=$R/private/tip2-src" \
  --declared "$R/docs/reports/rule50-consumer-composition/PREDICTION.md" \
  > "$OUT.log" 2>&1
echo $? > "$OUT.exit"
cat "$OUT.exit"
