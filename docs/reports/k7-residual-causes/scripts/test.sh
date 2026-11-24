#!/usr/bin/env bash
# test.sh NAME TEST [FILTER...] -- K7: build and run one qcore-c-gateway test
# target (or `lib` for the library unit tests) with the gate's test-build
# configuration into private/k7/target-test, in a `build` gate slot.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"; shift
test="$1"; shift
mkdir -p "$repo/private/k7"
cd "$repo" || exit 2
mapfile -t config < <(python3 scripts/test_link_config.py dynamic)
if [ "$test" = "lib" ]; then sel=(--lib); else sel=(--test "$test"); fi
CARGO_TARGET_DIR="$repo/private/k7/target-test" QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
  python3 scripts/gate_slot.py build --receipt "$repo/private/k7/$name.receipt.json" -- \
  cargo "${config[@]}" test -p qcore-c-gateway "${sel[@]}" -- --nocapture "$@" \
  > "$repo/private/k7/$name.log" 2>&1
echo $? > "$repo/private/k7/$name.exit"
