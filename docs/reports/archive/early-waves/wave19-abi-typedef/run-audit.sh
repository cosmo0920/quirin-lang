#!/bin/sh
# Canonical pinned-route 185-unit audit, run from this worktree
# (card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF, wave 19).
#
# Card CONSTRUCTOR-RESIDUALS' own script, with this card's provider-package and
# output directories. One detail is recorded rather than glossed: the pinned
# route reads `<source>/build/compile_commands.json`, which
# `scripts/prepare_cycle19_clang_audit.py` does not create — it configures the
# same pinned tree into `fluent-bit-clang-production`. That database's entries
# name the pinned source files and the pinned build's own generated-header
# paths, so it is copied into `<source>/build/` unchanged; the BEFORE and AFTER
# runs read the same bytes.
#
# Usage: run-audit.sh <out-subdir>
set -e
OUT="$1"
WT=<worktree>
PIN="$WT/build/cycle19/clang-pin"
mkdir -p "$WT/measure/runs/$OUT"
cd "$WT"
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE="$WT/build/cycle19/provider-packages-abi/qcore.chunkio" \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence \
  "$PIN/fluent-bit-clang-source" \
  "$PIN/monkey/monkey-build/compile_commands.json" \
  "$WT/measure/runs/$OUT/monkey-source.json" \
  "$WT/measure/runs/$OUT/resource-diagnostics.json"
