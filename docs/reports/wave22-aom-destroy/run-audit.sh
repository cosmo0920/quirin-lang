#!/bin/sh
# Canonical pinned-route 185-unit audit, run from this worktree
# (card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED, wave 22).
#
# Card ABI-TYPEDEF-ADDRESS-OF-MEMBER's script with this card's pin, provider
# package and output directories. The route detail those cards recorded holds:
# the pinned route reads `<source>/build/compile_commands.json`, which
# `scripts/prepare_cycle19_clang_audit.py` does not create, so the freshly
# prepared pin's own `fluent-bit-clang-production` database is copied there
# unchanged and the BEFORE and AFTER runs read the same bytes.
#
# Usage: run-audit.sh <out-subdir> <binary>
set -e
OUT="$1"
BIN="$2"
WT=<worktree>
PIN="$WT/build/cycle19/clang-pin"
mkdir -p "$WT/measure/runs/$OUT"
cd "$WT"
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE="$WT/build/cycle19/provider-packages-destroy/qcore.chunkio" \
"$BIN" \
  --pinned-evidence \
  "$PIN/fluent-bit-clang-source" \
  "$PIN/monkey/monkey-build/compile_commands.json" \
  "$WT/measure/runs/$OUT/monkey-source.json" \
  "$WT/measure/runs/$OUT/resource-diagnostics.json"
echo "AUDIT_EXIT=$?"
