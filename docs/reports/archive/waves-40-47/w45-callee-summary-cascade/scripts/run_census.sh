#!/bin/bash
# The run_audit.py command shape with a census-feature binary and the
# call-argument and callee-scope census streams on. Usage: run_census.sh BINARY OUT_DIR
PIN=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19
BINARY=$1
OUT=$(realpath -m "$2")
mkdir -p "$OUT"
export QCORE_CLANG=/usr/bin/clang
export QCORE_CHUNKIO_PACKAGE=$PIN/provider-packages-wave20/qcore.chunkio
unset QCORE_SOURCE_CONTRACTS QCORE_RETURN_CENSUS QCORE_INDIRECT_CENSUS_RECORDS QCORE_STATEMENT_CENSUS
export QCORE_CALL_ARGUMENT_CENSUS=$OUT/call-argument.jsonl
export QCORE_CALLEE_SCOPE_CENSUS=$OUT/callee-scope.jsonl
export QCORE_INDIRECT_CENSUS=$OUT/indirect.jsonl
"$BINARY" --pinned-evidence \
  "$PIN/c-gateway-wave-pin/fluent-bit-clang-source" \
  "$PIN/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
  "$OUT/monkey-source.json" "$OUT/resource-diagnostics.json" > "$OUT/audit.log" 2>&1
echo $? > "$OUT/exit"
