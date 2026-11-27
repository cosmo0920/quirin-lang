#!/usr/bin/env bash
# Card CORPUS-GROUP-CHECKPOINTS evidence runner: the pinned corpus with the
# measure corpus route's environment (link map, null property, runtime
# projection) and run_pinned_audit.py's (Clang, ChunkIO package), under an
# audit slot. Usage: run_corpus.sh OUT_DIR [pinned options...]
set -u
WT=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a50c061771c033423
INPUTS=/media/Data/GitHub/quirin-lang/build/cycle19
OUT=$1
shift
mkdir -p "$OUT"
# One working directory for every run: the route binds it into published rows.
WORK=$WT/private/evidence/work
mkdir -p "$WORK"
cd "$WORK"
python3 "$WT/scripts/gate_slot.py" audit --memory-budget 7G \
  --receipt "$OUT/slot.json" --wait-record "$OUT/slot-wait.json" -- \
  /usr/bin/time -v -o "$OUT/time.txt" \
  env -u QCORE_SOURCE_CONTRACTS \
    QCORE_CLANG=/usr/bin/clang \
    QCORE_CHUNKIO_PACKAGE="$INPUTS/provider-packages-wave20/qcore.chunkio" \
    QCORE_PROFILES="$WT/contract/profiles" \
    QCORE_CROSS_UNIT_LINK_MAP="$WT/docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json" \
    QCORE_NULL_PROPERTY="$OUT/corpus-null.jsonl" \
    QCORE_OUTER_PROJECTION="$OUT/corpus-outer.json" \
    SOURCE_DATE_EPOCH=1767225600 \
    "$WT/private/bin/quirin-audit" pinned --jobs 3 "$@" \
    "$INPUTS/c-gateway-wave-pin/fluent-bit-clang-source" \
    "$INPUTS/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
    "$OUT/monkey-source.json" "$OUT/resource-diagnostics.json" \
  > "$OUT/console.log" 2> "$OUT/stderr.log"
echo "exit $?" > "$OUT/exit.txt"
