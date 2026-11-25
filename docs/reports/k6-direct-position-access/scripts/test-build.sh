#!/usr/bin/env bash
# test-build.sh NAME TEST -- K6: build (no run) one gateway integration test
# (or `--lib`) in a `build` gate slot, target under private/k6/target-test.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
test="$2"
target="$repo/private/k6/target-test"
cd "$repo" || exit 2
if [ "$test" = "--lib" ]; then selector="--lib"; else selector="--test $test"; fi
CARGO_TARGET_DIR="$target" python3 scripts/gate_slot.py build \
  --receipt "$repo/private/k6/$name.receipt.json" -- \
  cargo test -p qcore-c-gateway $selector --no-run \
  > "$repo/private/k6/$name.log" 2>&1
status=$?
echo $status > "$repo/private/k6/$name.exit"
exit $status
