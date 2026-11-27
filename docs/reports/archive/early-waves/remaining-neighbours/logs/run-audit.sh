#!/bin/sh
# Canonical pinned-route 185-unit audit, run from this worktree.
# Usage: run-audit.sh <out-subdir>
set -e
OUT="$1"
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ad1192c365f7f1a48
PIN=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/clang/66910c10a4d7eafa810b84229cf2dcf7b0f26f97
mkdir -p "$WT/build/neighbours/$OUT"
cd "$WT"
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE="$WT/build/cycle19/provider-packages-neighbours/qcore.chunkio" \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence \
  "$PIN/fluent-bit-clang-source" \
  "$PIN/monkey/monkey-build/compile_commands.json" \
  "$WT/build/neighbours/$OUT/monkey-source.json" \
  "$WT/build/neighbours/$OUT/resource-diagnostics.json"
