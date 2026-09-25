#!/bin/sh
# usage: current.sh NAME JOBS [BINARY]
# Configured route (no --pinned-evidence) over the current-revision Clang build
# (build/cycle19/current-revision, source d1ab8abf) and the current Monkey
# inputs, into private/runs-current/slot, renamed to NAME.
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-af93c329e85457c34
build=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19
name=$1
jobs=$2
binary=${3:-$root/target/release/examples/cycle19_fluent_bit_libraries_audit}
runs="$root/private/runs-current"
mkdir -p "$runs" "$root/private/time"
if [ -e "$runs/slot" ] || [ -e "$runs/$name" ]; then
  echo "slot or $name exists" >&2
  exit 2
fi
mkdir "$runs/slot"
uptime > "$root/private/time/current-$name.load"
env -u QCORE_SOURCE_CONTRACTS -u QCORE_CHUNKIO_PACKAGE QCORE_CLANG=/usr/bin/clang QCORE_AUDIT_JOBS="$jobs" \
  /usr/bin/time -v -o "$root/private/time/current-$name.time" \
  "$binary" \
  --compile-database "$build/current-revision/fluent-bit-clang-production/compile_commands.json" \
  "$build/current-revision/fluent-bit-clang-source" \
  "$build/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
  "$runs/slot/monkey-source.json" \
  "$runs/slot/resource-diagnostics.json" \
  > "$runs/slot/audit.log" 2>&1
status=$?
echo "$status" > "$runs/slot/exit.txt"
sha256sum "$binary" > "$runs/slot/binary.sha256"
mv "$runs/slot" "$runs/$name"
exit $status
