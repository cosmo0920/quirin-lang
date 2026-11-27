#!/bin/sh
# Canonical pinned-route 185-unit audit, run from this worktree (card CONSTRUCTOR-RESIDUALS).
# Usage: run-audit.sh <out-subdir>
set -e
OUT="$1"
WT=<worktree>
PIN="$WT/build/cycle19/clang-pin"
mkdir -p "$WT/private/runs/$OUT"
cd "$WT"
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE="$WT/build/cycle19/provider-packages-cr/qcore.chunkio" \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence \
  "$PIN/fluent-bit-clang-source" \
  "$PIN/monkey/monkey-build/compile_commands.json" \
  "$WT/private/runs/$OUT/monkey-source.json" \
  "$WT/private/runs/$OUT/resource-diagnostics.json"
