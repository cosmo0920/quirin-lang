#!/bin/sh
# Usage: run_corpus.sh OUT_DIR  (run from the worktree root)
# The eight-home corpus route (contract/evaluations/eight-home) with the
# typed-path parity stream on, inputs read in place (read-only).
set -u
OUT="$1"
WT=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-abedc2c4207eed53b
mkdir -p "$OUT"
export QCORE_CROSS_UNIT_LINK_MAP="$WT/contract/evaluations/eight-home/link-map.json"
export QCORE_CLANG=/usr/lib/llvm-21/bin/clang
export QCORE_NULL_PROPERTY="$OUT/corpus-null.jsonl"
export QCORE_OUTER_PROJECTION="$OUT/corpus-outer.json"
# parity stream off: attempt 1 filled the shared disk (93 MB in 3 min)
unset QCORE_TYPED_PATH_PARITY
exec python3 "$WT/scripts/run_pinned_audit.py" "$OUT/corpus" 66910c10 \
  --inputs-root /media/Data/GitHub/quirin-lang/build/cycle19 --jobs 8
