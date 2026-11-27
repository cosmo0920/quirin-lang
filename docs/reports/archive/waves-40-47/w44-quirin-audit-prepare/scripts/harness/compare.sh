#!/bin/bash
set -u
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a0ab5a53f8a82118c
P=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-hungry-thompson-1740da/4e877459-f647-4ada-9799-501505fa7da8/scratchpad/parity
R=$(cat "$P/rev.txt")
name=$1
cd "$WT"
mkdir -p private/prep/compare
python3 docs/reports/w44-quirin-audit-prepare/scripts/compare_preparations.py \
  "$P/py/clang/$R" "$P/rs/clang/$R" "private/prep/compare/$name.json" \
  > "private/prep/compare/$name.out" 2>&1
echo $? > "private/prep/compare/$name.exit"
