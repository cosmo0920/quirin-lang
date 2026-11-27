#!/usr/bin/env bash
# unit-probe.sh BINARY UNIT.i FUNCTIONS CONTRACT.qv... -- K2: analyse named functions
# of one preprocessed production unit (preprocess.py) with the given contracts,
# through the scratch probe's `unit_probe` test; the parity stream goes to UNIT.i.parity.jsonl.
set -u
binary="$1"; unit="$2"; functions="$3"; shift 3
contracts="$(IFS=:; echo "$*")"
K2_UNIT_SOURCE="$unit" K2_UNIT_FUNCTIONS="$functions" K2_UNIT_CONTRACTS="$contracts" \
  QCORE_CLANG=/usr/lib/llvm-21/bin/clang QCORE_TYPED_PATH_PARITY="$unit.parity.jsonl" \
  "$binary" unit_probe --nocapture 2>&1 | grep -E -A2 "^UNIT|panicked" | cut -c1-1200
