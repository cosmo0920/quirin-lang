#!/bin/bash
# Same-path pass: the Python scripts and then `quirin-audit prepare --run`
# prepare into the SAME output path, one after the other, each followed by
# the same `current` audit; each result is moved aside before the next.
# No path normalisation is needed to compare them.
set -u
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a0ab5a53f8a82118c
S=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-hungry-thompson-1740da/4e877459-f647-4ada-9799-501505fa7da8/scratchpad/parity
FB=/home/cosmo/Documents/GitHub/fluent-bit
CLANG=/usr/lib/llvm-21/bin/clang
CLANGXX=/usr/lib/llvm-21/bin/clang++
BIN="$WT/target/release/quirin-audit"
cd "$WT"
rev=$(cat "$S/rev.txt")
out="$S/same/clang/$rev"
rm -rf "$S/same" "$S/same-py" "$S/same-rs"
python3 -B scripts/prepare_cmetrics_clang_audit.py --fluent-bit "$FB" \
  --revision "$rev" --output "$out" --clang "$CLANG" --clangxx "$CLANGXX" \
  > "$S/same-py-cmetrics.out" 2> "$S/same-py-cmetrics.err"
echo $? > "$S/same-py-cmetrics.exit"
python3 -B scripts/prepare_cycle19_monkey.py --fluent-bit "$out/fluent-bit-clang-source" \
  --output "$out/monkey" --clang "$CLANG" --clangxx "$CLANGXX" \
  > "$S/same-py-monkey.out" 2> "$S/same-py-monkey.err"
echo $? > "$S/same-py-monkey.exit"
mkdir -p "$out/reports"
QCORE_CLANG="$CLANG" "$BIN" current \
  --checkout "$out/fluent-bit-clang-source" \
  --profile-directory "$WT/contract/profiles/fluent-bit" \
  --compile-database "$out/fluent-bit-clang-production/compile_commands.json" \
  --monkey-database "$out/monkey/monkey-build/compile_commands.json" \
  --monkey-source-report "$out/reports/monkey-source.json" \
  --report "$out/reports/resource-diagnostics.json" --jobs 16 \
  > "$S/same-py-current.out" 2> "$S/same-py-current.err"
echo $? > "$S/same-py-current.exit"
mv "$S/same" "$S/same-py"
env -u QCORE_CLANG "$BIN" prepare --fluent-bit "$FB" --revision "$rev" \
  --output "$out" --clang "$CLANG" --clangxx "$CLANGXX" --run --jobs 16 \
  > "$S/same-rs-prepare-run.out" 2> "$S/same-rs-prepare-run.err"
echo $? > "$S/same-rs-prepare-run.exit"
mv "$S/same" "$S/same-rs"
echo done > "$S/same.done"
