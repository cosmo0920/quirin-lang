#!/bin/sh
# Card BASE-TYPE-READER-CENSUS: one pinned Cycle 19 corpus audit run with the
# base-type census streaming beside it. Same pin, compile database, ChunkIO
# package and environment as waves 33-36, so this run's verdict-bearing
# artifacts must reproduce docs/reports/joined-provenance-call-frames/artifacts/
# byte for byte. That is the card's inertness assertion.
#
# Usage: sh run_audit.sh <stage> <binary>
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a40aa758d95732b25
PIN=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin
PKG=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/provider-packages-wave20/qcore.chunkio
STAGE="$1"
BIN="$2"
[ -n "$STAGE" ] && [ -n "$BIN" ] || { echo "usage: run_audit.sh <stage> <binary>" >&2; exit 2; }
OUT="$WT/measure/base-type-reader-census/runs/$STAGE"
mkdir -p "$OUT"
unset QCORE_SOURCE_CONTRACTS
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE="$PKG" \
QCORE_INDIRECT_CENSUS="$OUT/indirect-leaves.jsonl" \
QCORE_INDIRECT_CENSUS_RECORDS="$OUT/record-bodies.jsonl" \
"$BIN" \
  --pinned-evidence \
  "$PIN/fluent-bit-clang-source" \
  "$PIN/monkey/monkey-build/compile_commands.json" \
  "$OUT/monkey-source.json" \
  "$OUT/resource-diagnostics.json" \
  >"$OUT/audit.log" 2>&1
echo "AUDIT_EXIT=$?" >"$OUT/exit.txt"
cat "$OUT/exit.txt"
