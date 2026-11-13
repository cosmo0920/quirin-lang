#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Card MEASURE-CONTRACT-ONLY-SIDE-WIRING: compare the harness's side
# directories with `quirin-audit measure` itself (run: sides), so the byte
# comparison and row comparison are the tool's own.
# Usage: compare.sh REPOSITORY RUNS OUT_PREFIX   (run from REPOSITORY)
set -u
REPO=$1 RUNS=$2 OUT=$3
for t in a b; do
  mkdir -p "$OUT-$t"
  "$REPO/target/release/quirin-audit" --plugin-path "$REPO/target/release" \
    --output "$OUT-$t" --timeout-ms 3600000 measure \
    --inputs "$REPO/docs/reports/measure-contract-only-side-wiring/evidence/inputs.json" \
    --inputs-base "$REPO" --repository "$REPO" --no-slots \
    --side "base=run:$RUNS/base" --side "composed=run:$RUNS/tip-$t" \
    --side "full=run:$RUNS/tip-$t-full" \
    --compare base composed --compare base full --compare composed full \
    > "$OUT-$t.log" 2>&1
  echo "tip-$t measure exit $?"
done
