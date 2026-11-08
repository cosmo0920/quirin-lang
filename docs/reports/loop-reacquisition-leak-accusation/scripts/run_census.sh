#!/bin/bash
# Card LOOP-REACQUISITION-LEAK-ACCUSATION: the leak-witness census on the two
# candidate units of the 046accf6 current run, through an audit slot.
set -u
W=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a9819016a66a78c54
C=/media/Data/GitHub/quirin-lang/build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source
OUT=$W/private/census/${1:-run}
mkdir -p "$OUT"
cd "$W"
export QCORE_CLANG=/usr/lib/llvm-21/bin/clang
export QCORE_LEAK_WITNESS=$OUT/leak-witness.jsonl
BIN=${2:-$W/private/census/quirin-audit-census}
python3 scripts/gate_slot.py audit --memory-budget 4G --receipt "$OUT/census.slot.json" -- \
  "$BIN" current \
  --checkout "$C/fluent-bit-clang-source" \
  --profile-directory "$W/contract/profiles/fluent-bit" \
  --compile-database "$C/fluent-bit-clang-production/compile_commands.json" \
  --selection "$W/private/census/snappy-selection.json" \
  --report "$OUT/resource-diagnostics.json" \
  --jobs 1 --memory-budget-mib 4096 > "$OUT/console.log" 2>&1
status=$?
echo "$status" > "$OUT/exit"
echo "exit $status"
