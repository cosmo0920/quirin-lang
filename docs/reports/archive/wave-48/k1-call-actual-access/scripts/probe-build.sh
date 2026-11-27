#!/usr/bin/env bash
# probe-build.sh NAME TEST [TARGET_DIR] -- K1: build (no run) one gateway
# integration test in a `build` gate slot, target under private/k1. Exits to files.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
test="$2"
target="$repo/private/k1/${3:-target-test}"
cd "$repo" || exit 2
CARGO_TARGET_DIR="$target" python3 scripts/gate_slot.py build \
  --receipt "$repo/private/k1/$name.receipt.json" -- \
  cargo test -p qcore-c-gateway $( [ "$test" = "--lib" ] && echo --lib || echo --test "$test" ) --no-run \
  > "$repo/private/k1/$name.log" 2>&1
echo $? > "$repo/private/k1/$name.exit"
