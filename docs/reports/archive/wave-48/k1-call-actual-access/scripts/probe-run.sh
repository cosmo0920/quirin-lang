#!/usr/bin/env bash
# probe-run.sh BINARY PROBE_FILE... -- K1: run the scratch probe test binary over
# probe files (one function per line), printing status and diagnostic keys.
set -u
binary="$1"
shift
for file in "$@"; do
  K1_PROBE_FILE="$file" QCORE_CLANG="${QCORE_CLANG:-/usr/lib/llvm-21/bin/clang}" \
    "$binary" --nocapture 2>&1 | grep -A1 -E "PROBE|panicked"
done
