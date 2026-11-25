#!/usr/bin/env bash
# probe-run.sh BINARY PARITY_OUT PROBE_FILE... -- K6: run the scratch probe test
# binary over probe files (one function `w` per line) with the typed-path
# parity stream written to PARITY_OUT, printing status and diagnostic keys.
set -u
binary="$1"
parity="$2"
shift 2
rm -f "$parity"
for file in "$@"; do
  QCORE_TYPED_PATH_PARITY="$parity" K6_PROBE_FILE="$file" \
    QCORE_CLANG="${QCORE_CLANG:-/usr/lib/llvm-21/bin/clang}" \
    "$binary" --nocapture 2>&1 | grep -A1 -E "PROBE|panicked"
done
