#!/bin/sh
# Canonical pinned-route 185-unit audit, run from this worktree
# (card W29-CORPUS-CONTRACTS, wave 29).
#
# Card GATEWAY-C-LOCATION's script with this card's FRESH pin, provider
# package and output directories. The route detail those cards recorded
# holds: the pinned route reads `<source>/build/compile_commands.json`, which
# `scripts/prepare_cycle19_clang_audit.py` does not create, so the freshly
# prepared pin's own `fluent-bit-clang-production` database is copied there
# unchanged and the BEFORE and AFTER runs read the same bytes.
#
# Usage: run-audit.sh <out-subdir> <binary>
set -eu
OUT="$1"
BIN="$2"
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a44c40b2d06b9644e
PIN="$WT/build/cycle19/clang-pin"
mkdir -p "$WT/measure/runs/$OUT"
cd "$WT"
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE="$WT/build/cycle19/provider-packages-w29/qcore.chunkio" \
"$BIN" \
  --pinned-evidence \
  "$PIN/fluent-bit-clang-source" \
  "$PIN/monkey/monkey-build/compile_commands.json" \
  "$WT/measure/runs/$OUT/monkey-source.json" \
  "$WT/measure/runs/$OUT/resource-diagnostics.json"
echo "AUDIT_EXIT=$?"
