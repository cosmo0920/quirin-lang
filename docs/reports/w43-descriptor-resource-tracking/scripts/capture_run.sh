#!/bin/bash
# The run_audit.py command shape, with QCORE_CLANG set to the transparent tee
# wrapper. Usage: capture_run.sh OUT_DIR
ROOT=$(cd "$(dirname "$0")/../../../.." && pwd)
PIN=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19
OUT=$1
mkdir -p "$OUT"
export QCORE_CLANG=$ROOT/docs/reports/w43-descriptor-resource-tracking/scripts/clang-tee.sh
export QCORE_CHUNKIO_PACKAGE=$PIN/provider-packages-wave20/qcore.chunkio
unset QCORE_SOURCE_CONTRACTS
"$ROOT/target/release/examples/cycle19_fluent_bit_libraries_audit" --pinned-evidence \
  "$PIN/c-gateway-wave-pin/fluent-bit-clang-source" \
  "$PIN/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
  "$OUT/monkey-source.json" "$OUT/resource-diagnostics.json" > "$OUT/audit.log" 2>&1
echo $? > "$OUT/exit"
