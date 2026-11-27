#!/bin/sh
# usage: run-current.sh NAME JOBS BINARY [EXTRA quirin-audit current ARGS...]
# `quirin-audit current` over the same prepared inputs as run-example.sh. The
# profile directory is spelled exactly as the example's profiles_root() spells
# it, so file-path contract keys are the same strings in every run.
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab16a94e01ece061d
build=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19
name=$1
jobs=$2
binary=$3
shift 3
runs="$root/private/runs"
mkdir -p "$runs"
if [ -e "$runs/$name" ]; then
  echo "$name exists" >&2
  exit 2
fi
mkdir "$runs/$name"
uptime > "$runs/$name/load.txt"
cd "$root" || exit 2
env -u QCORE_SOURCE_CONTRACTS -u QCORE_CHUNKIO_PACKAGE QCORE_CLANG=/usr/bin/clang \
  /usr/bin/time -v -o "$runs/$name/time.txt" \
  "$binary" current \
  --checkout "$build/current-revision/fluent-bit-clang-source" \
  --profile-directory "$root/compiler/../contract/profiles/fluent-bit" \
  --compile-database "$build/current-revision/fluent-bit-clang-production/compile_commands.json" \
  --monkey-database "$build/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
  --monkey-source-report "$runs/$name/monkey-source.json" \
  --report "$runs/$name/resource-diagnostics.json" \
  --jobs "$jobs" \
  "$@" \
  > "$runs/$name/audit.log" 2>&1
status=$?
echo "$status" > "$runs/$name/exit.txt"
sha256sum "$binary" > "$runs/$name/binary.sha256"
exit $status
