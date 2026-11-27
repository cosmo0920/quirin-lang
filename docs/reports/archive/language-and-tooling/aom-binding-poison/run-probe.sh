#!/bin/sh
# Read-only per-node probe over the pinned unit (card AOM-BINDING-POISON).
# Usage: run-probe.sh <binary> <function> <out>
set -eu
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a8c5f623cda3030bb
PIN="$WT/build/cycle19/clang-pin/fluent-bit-clang-source"
cd "$WT"
QCORE_CLANG=/usr/bin/clang "$1" \
  "$PIN/build/compile_commands.json" \
  "$PIN/src/flb_input_chunk.c" \
  "$2" > "$3" 2>&1
echo "PROBE_EXIT=$?"
