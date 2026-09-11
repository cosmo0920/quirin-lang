#!/bin/sh
# From the repository root: run-audit.sh PIN PACKAGE OUTPUT
# PIN holds the pinned Fluent Bit source and Monkey compile database. The
# source's build/compile_commands.json must be the recorded pinned database.
set -eu
PIN="$1"
PACKAGE="$2"
OUT="$3"
mkdir -p "$OUT"
QCORE_CLANG=/usr/bin/clang QCORE_CHUNKIO_PACKAGE="$PACKAGE" \
  cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence "$PIN/fluent-bit-clang-source" \
  "$PIN/monkey/monkey-build/compile_commands.json" \
  "$OUT/source.json" "$OUT/diagnostics.json"
