#!/bin/bash
# The run_audit.py command shape with QCORE_CLANG set to the transparent tee
# wrapper, for a private capture of the analysed -E -P unit texts.
# Usage: capture_run.sh BINARY OUT_DIR
ROOT=$(cd "$(dirname "$0")/../../../.." && pwd)
PIN=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19
BINARY=$1
OUT=$(realpath -m "$2")
mkdir -p "$OUT"
export QCORE_CAPTURE_DIR=$OUT
export QCORE_CLANG=$ROOT/docs/reports/w43-indexed-resource-collections/scripts/clang-tee.sh
export QCORE_CHUNKIO_PACKAGE=$PIN/provider-packages-wave20/qcore.chunkio
unset QCORE_SOURCE_CONTRACTS QCORE_RETURN_CENSUS QCORE_INDIRECT_CENSUS QCORE_INDIRECT_CENSUS_RECORDS QCORE_STATEMENT_CENSUS
"$BINARY" --pinned-evidence \
  "$PIN/c-gateway-wave-pin/fluent-bit-clang-source" \
  "$PIN/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
  "$OUT/monkey-source.json" "$OUT/resource-diagnostics.json" > "$OUT/audit.log" 2>&1
echo $? > "$OUT/exit"
