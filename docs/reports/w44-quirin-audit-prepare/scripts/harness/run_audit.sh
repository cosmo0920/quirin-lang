#!/bin/bash
# Second pass: rerun both preparations over their existing outputs (reuse
# path), the Rust one with --run; audit the Python preparation with the same
# `current` arguments the chained run uses.
set -u
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a0ab5a53f8a82118c
S=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-hungry-thompson-1740da/4e877459-f647-4ada-9799-501505fa7da8/scratchpad/parity
FB=/home/cosmo/Documents/GitHub/fluent-bit
CLANG=/usr/lib/llvm-21/bin/clang
CLANGXX=/usr/lib/llvm-21/bin/clang++
BIN="$WT/target/release/quirin-audit"
cd "$WT"
rev=$(cat "$S/rev.txt")
py_out="$S/py/clang/$rev"
rs_out="$S/rs/clang/$rev"
python3 -B scripts/prepare_cmetrics_clang_audit.py --fluent-bit "$FB" \
  --revision "$rev" --output "$py_out" --clang "$CLANG" --clangxx "$CLANGXX" \
  > "$S/py2-cmetrics.out" 2> "$S/py2-cmetrics.err"
echo $? > "$S/py2-cmetrics.exit"
python3 -B scripts/prepare_cycle19_monkey.py --fluent-bit "$py_out/fluent-bit-clang-source" \
  --output "$py_out/monkey" --clang "$CLANG" --clangxx "$CLANGXX" \
  > "$S/py2-monkey.out" 2> "$S/py2-monkey.err"
echo $? > "$S/py2-monkey.exit"
env -u QCORE_CLANG "$BIN" prepare --fluent-bit "$FB" --revision "$rev" \
  --output "$rs_out" --clang "$CLANG" --clangxx "$CLANGXX" --run --jobs 16 \
  > "$S/rs2-prepare-run.out" 2> "$S/rs2-prepare-run.err"
echo $? > "$S/rs2-prepare-run.exit"
mkdir -p "$py_out/reports"
QCORE_CLANG="$CLANG" "$BIN" current \
  --checkout "$py_out/fluent-bit-clang-source" \
  --profile-directory "$WT/contract/profiles/fluent-bit" \
  --compile-database "$py_out/fluent-bit-clang-production/compile_commands.json" \
  --monkey-database "$py_out/monkey/monkey-build/compile_commands.json" \
  --monkey-source-report "$py_out/reports/monkey-source.json" \
  --report "$py_out/reports/resource-diagnostics.json" --jobs 16 \
  > "$S/py2-current.out" 2> "$S/py2-current.err"
echo $? > "$S/py2-current.exit"
echo done > "$S/audit.done"
