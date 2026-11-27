#!/usr/bin/env bash
# usage: run_probes.sh <binary> <outdir>
# Prediction runs A (standalone Monkey, as the user's run reads it) and B
# (the linked Monkey configuration through an explicit unit selection) with
# one binary, the user's preparation and this worktree's profile, outputs
# under <outdir>. Each run dumps its member-facts pre-pass section.
set -u
W=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a97483da790d806af
P=/media/Data/GitHub/quirin-lang/build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736
BIN=$1
OUT=$2
mkdir -p "$OUT/A" "$OUT/B"
export QCORE_CLANG=/usr/lib/llvm-21/bin/clang
cd "$W"
QCORE_MEMBER_PREPASS_SECTION="$OUT/A/prepass-section.json" "$BIN" current \
  --checkout "$P/fluent-bit-clang-source" \
  --profile-directory "$W/contract/profiles/fluent-bit" \
  --compile-database "$P/fluent-bit-clang-production/compile_commands.json" \
  --monkey-database "$P/monkey/monkey-build/compile_commands.json" \
  --monkey-source-report "$OUT/A/monkey-source.json" \
  --selection "$W/docs/reports/current-monkey-linked-configuration/scripts/selection-standalone.json" \
  --report "$OUT/A/resource-diagnostics.json" \
  --jobs 6 --memory-budget-mib 7168 > "$OUT/A/console.log" 2> "$OUT/A/stderr.log"
echo $? > "$OUT/A/exit.txt"
QCORE_MEMBER_PREPASS_SECTION="$OUT/B/prepass-section.json" "$BIN" current \
  --checkout "$P/fluent-bit-clang-source" \
  --profile-directory "$W/contract/profiles/fluent-bit" \
  --compile-database "$P/fluent-bit-clang-production/compile_commands.json" \
  --selection "$W/docs/reports/current-monkey-linked-configuration/scripts/selection-linked.json" \
  --report "$OUT/B/resource-diagnostics.json" \
  --jobs 6 --memory-budget-mib 7168 > "$OUT/B/console.log" 2> "$OUT/B/stderr.log"
echo $? > "$OUT/B/exit.txt"
