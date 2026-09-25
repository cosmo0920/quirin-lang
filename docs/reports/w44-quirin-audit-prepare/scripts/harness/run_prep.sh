#!/bin/bash
# Parity preparation: Python scripts vs `quirin-audit prepare`, same local
# Fluent Bit revision, two fresh output roots outside the worktree.
set -u
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a0ab5a53f8a82118c
S=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-hungry-thompson-1740da/4e877459-f647-4ada-9799-501505fa7da8/scratchpad/parity
FB=/home/cosmo/Documents/GitHub/fluent-bit
CLANG=/usr/lib/llvm-21/bin/clang
CLANGXX=/usr/lib/llvm-21/bin/clang++
cd "$WT"
rev=$(git -C "$FB" rev-parse HEAD)
echo "$rev" > "$S/rev.txt"
py_out="$S/py/clang/$rev"
rs_out="$S/rs/clang/$rev"
(
  python3 -B scripts/prepare_cmetrics_clang_audit.py --fluent-bit "$FB" \
    --revision "$rev" --output "$py_out" --clang "$CLANG" --clangxx "$CLANGXX" \
    > "$S/py-cmetrics.out" 2> "$S/py-cmetrics.err"
  echo $? > "$S/py-cmetrics.exit"
  python3 -B scripts/prepare_cycle19_monkey.py --fluent-bit "$py_out/fluent-bit-clang-source" \
    --output "$py_out/monkey" --clang "$CLANG" --clangxx "$CLANGXX" \
    > "$S/py-monkey.out" 2> "$S/py-monkey.err"
  echo $? > "$S/py-monkey.exit"
) &
py=$!
(
  "$WT/target/debug/quirin-audit" prepare --fluent-bit "$FB" --revision "$rev" \
    --output "$rs_out" --clang "$CLANG" --clangxx "$CLANGXX" \
    > "$S/rs-prepare.out" 2> "$S/rs-prepare.err"
  echo $? > "$S/rs-prepare.exit"
) &
rs=$!
wait "$py" "$rs"
echo done > "$S/prep.done"
