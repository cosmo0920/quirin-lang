#!/bin/bash
set -u
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a0ab5a53f8a82118c
P=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-hungry-thompson-1740da/4e877459-f647-4ada-9799-501505fa7da8/scratchpad/parity
R=$(cat "$P/rev.txt")
cd "$WT"
mkdir -p private/prep/compare
python3 docs/reports/w44-quirin-audit-prepare/scripts/compare_audits.py \
  "$P/py/clang/$R" "$P/rs/clang/$R" "$P/py2-current.out" "$P/rs2-prepare-run.out" \
  private/prep/compare/audit.json > private/prep/compare/audit.out 2>&1
echo $? > private/prep/compare/audit.exit
