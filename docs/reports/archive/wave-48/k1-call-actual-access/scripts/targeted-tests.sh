#!/usr/bin/env bash
# targeted-tests.sh -- K1: run the built fixture test and the filtered gateway
# unit tests (private/k1/target-test, built by probe-build.sh); logs and exits
# under private/k1/.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
deps="$repo/private/k1/target-test/debug/deps"
export QCORE_CLANG="${QCORE_CLANG:-/usr/lib/llvm-21/bin/clang}"
cd "$repo/crates/qcore-c-gateway" || exit 2
lib="$(ls -t "$deps"/qcore_c_gateway-* | grep -v '\.d$' | head -1)"
fixture="$(ls -t "$deps"/k1_call_actual_access-* | grep -v '\.d$' | head -1)"
"$lib" parenthesised_member_operand lexical_nested_assignment logical_and_operand borrowed_actual \
  > "$repo/private/k1/lib-tests.log" 2>&1
echo $? > "$repo/private/k1/lib-tests.exit"
"$fixture" > "$repo/private/k1/fixture-tests.log" 2>&1
echo $? > "$repo/private/k1/fixture-tests.exit"
