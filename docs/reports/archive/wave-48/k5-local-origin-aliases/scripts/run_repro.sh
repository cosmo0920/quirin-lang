#!/usr/bin/env bash
# Unit-scoped reproduction of the user's `current --monkey-linked` run.
#
# usage: run_repro.sh QUIRIN_AUDIT_BINARY FILTERED_COMPILE_COMMANDS.json OUT_DIR
#
# The user's argv (build/.../reports/resource-diagnostics.invocation.json)
# with the Fluent Bit compile database replaced by the filtered one, the
# profile directory of this checkout, and both outputs written under OUT_DIR;
# every input is read in place, nothing under build/ is written. Run through
# scripts/gate_slot.py audit.
set -euo pipefail
binary=$1
database=$2
out=$3
prepared=/media/Data/GitHub/quirin-lang/build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source
root=$(cd "$(dirname "$0")/../../../.." && pwd)
mkdir -p "$out"
exec "$binary" current \
  --checkout "$prepared/fluent-bit-clang-source" \
  --profile-directory "$root/contract/profiles/fluent-bit" \
  --compile-database "$database" \
  --monkey-database "$prepared/monkey/monkey-build/compile_commands.json" \
  --monkey-linked \
  --monkey-source-report "$out/monkey-source.json" \
  --report "$out/resource-diagnostics.json" \
  --jobs 3
