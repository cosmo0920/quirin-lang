#!/usr/bin/env bash
# unit-probe.sh BINARY PARITY_OUT UNIT_I FUNCS -- K6: analyse the comma-separated
# FUNCS of one preprocessed unit (preprocess.py output) with the scratch probe's
# `unit_probe` over the generic fixture contracts, parity stream to PARITY_OUT.
# A fixture-contract approximation of the unit-scoped run, for refusal reasons only.
set -u
binary="$1"
parity="$2"
unit="$3"
funcs="$4"
rm -f "$parity"
QCORE_TYPED_PATH_PARITY="$parity" K6_UNIT_FILE="$unit" K6_UNIT_FUNCS="$funcs" \
  QCORE_CLANG="${QCORE_CLANG:-/usr/lib/llvm-21/bin/clang}" \
  "$binary" unit_probe --nocapture 2>&1 | grep -E "^UNIT|panicked"
