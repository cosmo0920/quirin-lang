#!/bin/sh
# The AUDIT-CANONICAL-PATHS identity comparisons for one route.
# Usage: compare_all.sh WORKTREE RUNS_DIR_NAME ROUTE BASE_SNAP TIP_MIRROR_SNAP TIP_ORIGINAL_SNAP BASE_NS_SNAP
# Writes private/acp/cmp/<runs>-<route>-<pair>.json and prints each summary.
set -u
W=$1
RUNS=$2
ROUTE=$3
A=$W/private/acp
R=$A/$RUNS
OUT=$A/cmp
C=$W/docs/reports/audit-canonical-paths/scripts/compare.py
REAL=/media/Data/GitHub/quirin-lang
OLD=/home/cosmo/Documents/GitHub/qcore-c
M=$A/mirror
BASE=$A/$4
TIPM=$A/$5
TIPO=$A/$6
BNS=$A/$7
mkdir -p "$OUT"
PIN=build/cycle19/c-gateway-wave-pin
case $ROUTE in
  corpus)
    # TIP-original spells the database's old prefix and the resolved inputs
    # root; the mirror spells its copies (source, Monkey, chunkio package)
    # under M and every other old-prefix path under REAL.
    ORIGINAL_TO_MIRROR="--sub $OLD/$PIN/fluent-bit-clang-source=$M/c-gateway-wave-pin/fluent-bit-clang-source
      --sub $OLD/$PIN/monkey=$M/c-gateway-wave-pin/monkey
      --sub $REAL/$PIN/fluent-bit-clang-source=$M/c-gateway-wave-pin/fluent-bit-clang-source
      --sub $REAL/$PIN/monkey=$M/c-gateway-wave-pin/monkey
      --sub $REAL/build/cycle19/provider-packages-wave20=$M/provider-packages-wave20
      --sub $OLD/build/cycle19=$REAL/build/cycle19"
    ORIGINAL_TO_NAMESPACE="--sub $REAL/build/cycle19=$OLD/build/cycle19"
    ;;
  system)
    P=private/w45-fb/w46-external-callee-contracts/attempt-1
    ORIGINAL_TO_MIRROR="--sub $REAL/$P/fluent-bit-clang-production/compile_commands.json=$M/system-compile_commands.json
      --sub $OLD/$P=$REAL/$P"
    ORIGINAL_TO_NAMESPACE="--sub $REAL/$P=$OLD/$P"
    ;;
  slice)
    P=private/w47-tail/attempt-1/prep
    ORIGINAL_TO_MIRROR="--sub $REAL/$P/fluent-bit-clang-production/compile_commands.json=$M/slice-compile_commands.json
      --sub $OLD/$P=$REAL/$P"
    ORIGINAL_TO_NAMESPACE="--sub $REAL/$P=$OLD/$P"
    ;;
esac
python3 "$C" "$ROUTE" "$R/base-mirror/$ROUTE" "$R/tip-mirror/$ROUTE" "$OUT/$RUNS-$ROUTE-base-mirror-vs-tip-mirror.json" \
  --sub "$TIPM=$BASE"
echo "exit $?"
# shellcheck disable=SC2086
python3 "$C" "$ROUTE" "$R/base-mirror/$ROUTE" "$R/tip-original/$ROUTE" "$OUT/$RUNS-$ROUTE-base-mirror-vs-tip-original.json" \
  --sub "$TIPO=$BASE" $ORIGINAL_TO_MIRROR
echo "exit $?"
# shellcheck disable=SC2086
python3 "$C" "$ROUTE" "$R/base-namespace/$ROUTE" "$R/tip-original/$ROUTE" "$OUT/$RUNS-$ROUTE-base-namespace-vs-tip-original.json" \
  --sub "$TIPO=$BNS" $ORIGINAL_TO_NAMESPACE
echo "exit $?"
