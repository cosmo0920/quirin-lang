#!/bin/bash
set -u
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a0ab5a53f8a82118c
P=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-hungry-thompson-1740da/4e877459-f647-4ada-9799-501505fa7da8/scratchpad/parity
R=$(cat "$P/rev.txt")
cd "$WT"
mkdir -p private/prep/compare
python3 docs/reports/w44-quirin-audit-prepare/scripts/compare_preparations.py \
  "$P/same-py/clang/$R" "$P/same-rs/clang/$R" private/prep/compare/same-path.json \
  > private/prep/compare/same-path.out 2>&1
echo $? > private/prep/compare/same-path.exit
python3 docs/reports/w44-quirin-audit-prepare/scripts/compare_audits.py \
  "$P/same-py/clang/$R" "$P/same-rs/clang/$R" "$P/same-py-current.out" "$P/same-rs-prepare-run.out" \
  private/prep/compare/same-path-audit.json > private/prep/compare/same-path-audit.out 2>&1
echo $? > private/prep/compare/same-path-audit.exit
