#!/usr/bin/env bash
# c03_census.sh NAME -- re-read C03-NFR-REACH (CG27-LOOPS-RECURSION-BUDGETS):
# run C03-REACH's read-only recursive-component census over the pinned
# prepared clone the arbiter reads, into private/cg27lrb-c03/census-NAME.json.
# The census publishes no verdict and writes no audit artifact. Every exit is
# written to a file; nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg27lrb-c03"
inputs=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
  timeout 7200 cargo run --quiet -p qcore-compiler --example c03_reach_census -- \
  "$inputs/fluent-bit-clang-source" \
  "$inputs/monkey/monkey-build/compile_commands.json" \
  "$out/census-$name.json" > "$out/census-$name.log" 2>&1
echo $? > "$out/census-$name.exit"
