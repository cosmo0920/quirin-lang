#!/bin/sh
# system.sh SIDE_TREE OUT -- the fluent-bit-system route (the evaluation
# profile's command: `current --selection system-selection.json`, the
# eight-home link map, null and outer projections on), as
# docs/reports/audit-canonical-paths/scripts/measure.py runs it, over this
# host's 046accf6 preparation (private/fb-046accf6) and the side tree's own
# release quirin-audit.
TREE=$(cd "$1" && pwd)
OUT=$2
P=/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
ENV=/home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
unset QCORE_SOURCE_CONTRACTS QCORE_RETURN_CENSUS QCORE_CHUNKIO_PACKAGE
export QCORE_PROFILES="$TREE/contract/profiles"
export QCORE_CROSS_UNIT_LINK_MAP="$TREE/contract/evaluations/eight-home/link-map.json"
export QCORE_CLANG=/usr/lib/llvm-21/bin/clang
export QCORE_NULL_PROPERTY="$OUT/system-null.jsonl"
export QCORE_OUTER_PROJECTION="$OUT/system-outer.json"
export SOURCE_DATE_EPOCH=1767225600
date -u +%FT%TZ > "$OUT/started"
sh "$ENV" "$TREE/target/release/quirin-audit" current \
  --checkout "$P/fluent-bit-clang-source" \
  --profile-directory "$TREE/contract/profiles/fluent-bit" \
  --compile-database "$P/fluent-bit-clang-production/compile_commands.json" \
  --selection "$TREE/contract/profiles/fluent-bit/system-selection.json" \
  --jobs 12 --report "$OUT/resource-diagnostics.json" > "$OUT/console.log" 2> "$OUT/stderr.log"
echo $? > "$OUT/exit"
date -u +%FT%TZ > "$OUT/finished"
