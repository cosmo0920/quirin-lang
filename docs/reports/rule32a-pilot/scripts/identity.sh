#!/bin/sh
# RULE32A-PILOT: the identity run on the three live profiles, in one
# `quirin-audit measure` run: BASE (the published `baseline:db2795100`, the
# landing publication of origin/main db2795100, every file digest, the route
# definition, the inputs and the contract-bundle digests verified by the tool)
# against TIP (`rev:` the card's code commit, built by the tool at its fixed
# path, under the baseline's build-commit label so the reports spell the same
# build commit; the side record keeps the revision), checked against PREDICTION.md (`--declared`: no row moves). No
# `--rules-off` side: rule 32a is a fixed hold (`off = "not-switchable"`), so
# the analyzer refuses `QCORE_GATEWAY_RULES_OFF=32a` and there is no switch to
# measure. The wall-time quota is 24 h because earlier pilots' runs took
# 10-14 h under slot contention.
# Usage: identity.sh WORKTREE OUT_DIR TIP_REV   (OUT_DIR fresh)
set -u
W=$1
OUT=$2
TIP=$3
cd "$W" || exit 2
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.start"
df -h /media/Data >"$OUT.df-start"
CARGO_BUILD_JOBS=6 cargo build --release -p qcore-audit -p quirin-audit-plugin-measure \
  >"$OUT.build.log" 2>&1
echo $? >"$OUT.build.exit"
"$W/target/release/quirin-audit" --plugin-path "$W/target/release" --output "$OUT" \
  --timeout-ms 86400000 measure \
  --profile eight-home --profile fluent-bit-system --profile fluent-bit-tail-pause \
  --side base=baseline:db2795100a760efa0a2cbcbe13217bffc3cf4733 \
  --side tip=rev:"$TIP" --build-commit db2795100a760efa0a2cbcbe13217bffc3cf4733 \
  --declared docs/reports/rule32a-pilot/PREDICTION.md \
  --baselines /media/Data/GitHub/quirin-lang/private/baselines \
  --inputs-base /media/Data/GitHub/quirin-lang --repository . >"$OUT.log" 2>&1
echo $? >"$OUT.exit"
df -h /media/Data >"$OUT.df-end"
date -u +%Y-%m-%dT%H:%M:%SZ >"$OUT.end"
