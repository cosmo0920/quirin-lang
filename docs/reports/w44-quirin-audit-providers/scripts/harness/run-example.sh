#!/bin/sh
# usage: run-example.sh NAME JOBS BINARY
# The configured route of cycle19_fluent_bit_libraries_audit (no
# --pinned-evidence) over the prepared current-revision Clang build and the
# pinned Monkey Clang database, exactly as W44-AUDIT-PARALLEL-UNITS ran it.
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab16a94e01ece061d
build=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19
name=$1
jobs=$2
binary=$3
runs="$root/private/runs"
mkdir -p "$runs"
if [ -e "$runs/$name" ]; then
  echo "$name exists" >&2
  exit 2
fi
mkdir "$runs/$name"
uptime > "$runs/$name/load.txt"
env -u QCORE_SOURCE_CONTRACTS -u QCORE_CHUNKIO_PACKAGE QCORE_CLANG=/usr/bin/clang QCORE_AUDIT_JOBS="$jobs" \
  /usr/bin/time -v -o "$runs/$name/time.txt" \
  "$binary" \
  --compile-database "$build/current-revision/fluent-bit-clang-production/compile_commands.json" \
  "$build/current-revision/fluent-bit-clang-source" \
  "$build/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
  "$runs/$name/monkey-source.json" \
  "$runs/$name/resource-diagnostics.json" \
  > "$runs/$name/audit.log" 2>&1
status=$?
echo "$status" > "$runs/$name/exit.txt"
sha256sum "$binary" > "$runs/$name/binary.sha256"
exit $status
