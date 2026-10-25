#!/bin/sh
# Attempt-2 identity comparisons for one route (every side at the fixed
# snapshot and output paths, so only input spellings can differ).
# Usage: compare_runs2.sh WORKTREE ROUTE [PAIRS]
#   PAIRS: any of mirror original namespace (default all three)
#   mirror     base-mirror vs tip-mirror, no substitution
#   original   base-mirror vs tip-original, TIP's input spellings rewritten to the mirror's
#   namespace  base-namespace vs tip-original, TIP's resolved roots rewritten to the old spelling
#   (NS_RUNS=runs3 takes the namespace side from the sibling-preserving rerun)
set -u
W=$1
ROUTE=$2
A=$W/private/acp
R=$A/runs2
OUT=$W/docs/reports/audit-canonical-paths/identity
C=$W/docs/reports/audit-canonical-paths/scripts/compare.py
REAL=/media/Data/GitHub/quirin-lang
OLD=/home/cosmo/Documents/GitHub/qcore-c
M=$A/mirror
PIN=build/cycle19/c-gateway-wave-pin
mkdir -p "$OUT"
case $ROUTE in
  corpus)
    TO_MIRROR="--sub $OLD/$PIN/fluent-bit-clang-source=$M/c-gateway-wave-pin/fluent-bit-clang-source
      --sub $OLD/$PIN/monkey=$M/c-gateway-wave-pin/monkey
      --sub $REAL/$PIN/fluent-bit-clang-source=$M/c-gateway-wave-pin/fluent-bit-clang-source
      --sub $REAL/$PIN/monkey=$M/c-gateway-wave-pin/monkey
      --sub $REAL/build/cycle19/provider-packages-wave20=$M/provider-packages-wave20
      --sub $OLD/build/cycle19=$REAL/build/cycle19"
    TO_OLD="--sub $REAL/build/cycle19=$OLD/build/cycle19" ;;
  system)
    P=private/w45-fb/w46-external-callee-contracts/attempt-1
    TO_MIRROR="--sub $REAL/$P/fluent-bit-clang-production/compile_commands.json=$M/system-compile_commands.json
      --sub $OLD/$P=$REAL/$P"
    TO_OLD="--sub $REAL/$P=$OLD/$P" ;;
  slice)
    P=private/w47-tail/attempt-1/prep
    TO_MIRROR="--sub $REAL/$P/fluent-bit-clang-production/compile_commands.json=$M/slice-compile_commands.json
      --sub $OLD/$P=$REAL/$P"
    TO_OLD="--sub $REAL/$P=$OLD/$P" ;;
esac
for pair in ${3:-mirror original namespace}; do
  case $pair in
    mirror) python3 "$C" "$ROUTE" "$R/base-mirror/$ROUTE" "$R/tip-mirror/$ROUTE" "$OUT/$ROUTE-base-mirror-vs-tip-mirror.json" ;;
    # shellcheck disable=SC2086
    original) python3 "$C" "$ROUTE" "$R/base-mirror/$ROUTE" "$R/tip-original/$ROUTE" "$OUT/$ROUTE-base-mirror-vs-tip-original.json" $TO_MIRROR ;;
    # shellcheck disable=SC2086
    namespace) python3 "$C" "$ROUTE" "$A/${NS_RUNS:-runs2}/base-namespace/$ROUTE" "$R/tip-original/$ROUTE" "$OUT/$ROUTE-base-namespace-vs-tip-original.json" $TO_OLD ;;
  esac
  echo "$pair exit $?"
done
