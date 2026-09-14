#!/bin/sh
# Card C-GATEWAY-RETURN-POSITION: one pinned Cycle 19 corpus audit run.
#
# Usage: sh measure/c-gateway-return-position/run-audit.sh <out-subdir> <binary>
#
# The pin, the compile database, the ChunkIO package and the two environment
# variables are exactly the ones the wave-31 chain used, so a BEFORE run of the
# base binary must reproduce the current baseline archives byte for byte. The
# transport stays `HistoricalStdin` (constraint 11) and is not selected here:
# it is the audit example's own pinned-evidence route.
set -e
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a310b80d397bfb2a1
PIN=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin
PKG=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/provider-packages-wave20/qcore.chunkio
OUT="$1"
BIN="$2"
[ -n "$OUT" ] && [ -n "$BIN" ] || { echo "usage: run-audit.sh <out-subdir> <binary>" >&2; exit 2; }
mkdir -p "$WT/measure/c-gateway-return-position/runs/$OUT"
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE="$PKG" \
"$BIN" \
  --pinned-evidence \
  "$PIN/fluent-bit-clang-source" \
  "$PIN/monkey/monkey-build/compile_commands.json" \
  "$WT/measure/c-gateway-return-position/runs/$OUT/monkey-source.json" \
  "$WT/measure/c-gateway-return-position/runs/$OUT/resource-diagnostics.json" \
  >"$WT/measure/c-gateway-return-position/runs/$OUT/audit.log" 2>&1
echo "AUDIT_EXIT=$?"
