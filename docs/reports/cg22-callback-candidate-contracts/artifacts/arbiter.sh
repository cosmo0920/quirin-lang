#!/bin/sh
# arbiter.sh OUT BASE_SPEC TIP_SPEC [extra measure args...]: quirin-audit
# measure, corpus route, on this card's preparations (artifacts/inputs.json),
# with the measure plugin built from the tip side's tree (private/tip-src).
# BEFORE census: arbiter.sh measure/before base=private/base-src again=private/base-src
# Arbiter: arbiter.sh measure/arbiter-N base=run:measure/before/sides/base tip=private/tip-src --declared .../PREDICTION.md
R=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a5d0fa65ce9b509f2
cd "$R" || exit 2
OUT=$1; A=$2; B=$3; shift 3
P=${PLUGIN_SRC:-$R/private/tip-src}
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh \
  "$P/target/release/quirin-audit" --plugin-path "$P/target/release" \
  --output "$OUT" --timeout-ms 36000000 measure \
  --inputs "${INPUTS:-$R/docs/reports/cg22-callback-candidate-contracts/artifacts/inputs.json}" \
  --inputs-base "$R" --repository "$R" --routes corpus --no-slots --jobs 8 \
  --side "$A" --side "$B" "$@" \
  > "$OUT.log" 2>&1
echo $? > "$OUT.exit"
cat "$OUT.exit"
