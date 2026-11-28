#!/bin/sh
# suite-census.sh TREE OUT -- run the qcore-c-gateway test suite of TREE (a
# `git archive` of the measured revision, with third_party/libevent extracted
# from its submodule archive) with the `indirect-census` feature, appending
# rule 84's QCORE_MARK_CENSUS lines (one per marked current-generation
# (exit, identity) of an analysis with no Unknown leaf at the settle point) to
# OUT/mark-census.jsonl. Card SETTLED-LEAK-MARK-LEAFLESS.
TREE=$(cd "$1" && pwd)
mkdir -p "$2"
OUT=$(cd "$2" && pwd)
ENV=/home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
cd "$TREE" || exit 2
QCORE_MARK_CENSUS="$OUT/mark-census.jsonl"
export QCORE_MARK_CENSUS
rm -f "$QCORE_MARK_CENSUS"
date -u +%FT%TZ > "$OUT/started"
sh "$ENV" cargo test -p qcore-c-gateway --features indirect-census --no-fail-fast \
  > "$OUT/test.log" 2>&1
echo $? > "$OUT/exit"
date -u +%FT%TZ > "$OUT/finished"
