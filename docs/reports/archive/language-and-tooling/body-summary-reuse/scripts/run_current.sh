#!/usr/bin/env bash
# usage: run_current.sh <outdir> [link-map]
# The user's `current` invocation (resource-diagnostics.invocation.json argv),
# with this worktree's base binary and profile and outputs under <outdir>.
set -u
W=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a57f3507d9d7199a0
S=/media/Data/GitHub/quirin-lang/build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source
OUT=$1
mkdir -p "$OUT"
if [ $# -ge 2 ]; then export QCORE_CROSS_UNIT_LINK_MAP=$2; fi
export QCORE_CLANG=/usr/lib/llvm-21/bin/clang
cd "$W"
python3 scripts/gate_slot.py audit --memory-budget "${BUDGET:-7G}" --receipt "$OUT/slot-receipt.json" -- \
  "$W/target/release/quirin-audit" current \
  --checkout "$S/fluent-bit-clang-source" \
  --profile-directory "$W/contract/profiles/fluent-bit" \
  --compile-database "$S/fluent-bit-clang-production/compile_commands.json" \
  --monkey-database "$S/monkey/monkey-build/compile_commands.json" \
  --monkey-source-report "$OUT/monkey-source.json" \
  --report "$OUT/resource-diagnostics.json" \
  --jobs 8 > "$OUT/console.log" 2> "$OUT/stderr.log"
echo $? > "$OUT/exit.txt"
