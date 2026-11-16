#!/bin/sh
# arbiter.sh OUT [ROUTES]: quirin-audit measure --declared over the corpus and system
# routes, base export (d7a497822) against tip export (238c66cf2), on this
# card's preparations (artifacts/inputs.json). Exit to OUT.exit, log to
# OUT.log; nothing is piped.
R=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a717a6f321d7ff5d9
P=$R/private/w48-libcurl
OUT=$1
ROUTES=${2:-corpus,system}
cd "$R" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh \
  "$P/tip-src/target/release/quirin-audit" --plugin-path "$P/tip-src/target/release" \
  --output "$OUT" --timeout-ms 36000000 measure \
  --inputs "$R/docs/reports/w48-libcurl-contract-plugin/artifacts/inputs.json" \
  --inputs-base "$R" --repository "$R" --routes "$ROUTES" --jobs 3 \
  --side "base=$P/base-src" --side "tip=$P/tip-src" \
  --declared "$R/docs/reports/w48-libcurl-contract-plugin/PREDICTION.md" \
  > "$OUT.log" 2>&1
echo $? > "$OUT.exit"
cat "$OUT.exit"
