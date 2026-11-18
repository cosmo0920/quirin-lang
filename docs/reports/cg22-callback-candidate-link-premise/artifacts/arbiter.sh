#!/bin/sh
# arbiter.sh OUT INPUTS SIDE_A SIDE_B [extra measure args...]: quirin-audit
# measure, corpus route, on this card's preparation, with the host and the
# measure plugin of the base export (private/base-src; the card changes no
# analyzer or plugin code). Adapted from
# docs/reports/cg22-callback-candidate-contracts/artifacts/arbiter.sh.
#
# BEFORE (eight-home map, stability):
#   arbiter.sh measure/before artifacts/inputs.eight-home.json \
#     base=private/base-src again=private/base-src
# Arbiter (twelve-home map against BEFORE's base side, one binary):
#   arbiter.sh measure/arbiter-1 artifacts/inputs.twelve-home.json \
#     base=run:measure/before/sides/base tip=private/base-src \
#     --declared docs/reports/cg22-callback-candidate-link-premise/PREDICTION.md
R=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a83f807263a0b65f2
cd "$R" || exit 2
OUT=$1; INPUTS=$2; A=$3; B=$4; shift 4
P=$R/private/base-src
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh \
  "$P/target/release/quirin-audit" --plugin-path "$P/target/release" \
  --output "$OUT" --timeout-ms 36000000 measure \
  --inputs "$INPUTS" --inputs-base "$R" --repository "$R" --routes corpus \
  --no-slots --jobs 8 --side "$A" --side "$B" "$@" \
  > "$OUT.log" 2>&1
echo $? > "$OUT.exit"
cat "$OUT.exit"
