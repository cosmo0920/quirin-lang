#!/bin/sh
# Canonical pinned-route 185-unit audit, run from this worktree (card ALIAS-AGGREGATE).
# Usage: run-audit.sh <out-subdir>
set -e
OUT="$1"
WT=<worktree>
PIN="$WT/measure/clang66910c10"
mkdir -p "$WT/measure/runs/$OUT"
cd "$WT"
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE="$WT/build/cycle19/provider-packages-alias-aggregate/qcore.chunkio" \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence \
  "$PIN/fluent-bit-clang-source" \
  "$PIN/monkey/monkey-build/compile_commands.json" \
  "$WT/measure/runs/$OUT/monkey-source.json" \
  "$WT/measure/runs/$OUT/resource-diagnostics.json"
