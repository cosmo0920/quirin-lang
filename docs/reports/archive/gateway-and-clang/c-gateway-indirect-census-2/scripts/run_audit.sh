#!/bin/sh
# Card C-GATEWAY-INDIRECT-CENSUS-2: one pinned Cycle 19 corpus audit run, with
# the instrumented indirect-access census streaming beside it.
#
# The pin, the compile database, the ChunkIO package and the two environment
# variables are exactly the ones waves 33-35 used, so this run's verdict-bearing
# artifacts must reproduce `docs/reports/c-gateway-constant-do-while/artifacts/`
# byte for byte. That is the card's inertness assertion: the census reads the
# analyzer's state at a refusal it has already decided, and changes nothing.
#
# Usage: sh measure/c-gateway-indirect-census-2/run_audit.sh <stage> <binary>
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a1204a1a083d35165
PIN=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin
PKG=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/provider-packages-wave20/qcore.chunkio
STAGE="$1"
BIN="$2"
[ -n "$STAGE" ] && [ -n "$BIN" ] || { echo "usage: run_audit.sh <stage> <binary>" >&2; exit 2; }
OUT="$WT/measure/c-gateway-indirect-census-2/runs/$STAGE"
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
ls -l "$OUT"
