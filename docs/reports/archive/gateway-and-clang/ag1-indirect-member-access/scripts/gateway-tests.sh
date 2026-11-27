#!/usr/bin/env bash
# gateway-tests.sh NAME -- AG1: every qcore-c-gateway integration test, no
# fail-fast, in a `build` gate slot (a targeted run before the card gate).
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
cd "$repo" || exit 2
python3 scripts/gate_slot.py build --receipt "$repo/private/ag1/$name.receipt.json" -- \
  cargo test -p qcore-c-gateway --tests --no-fail-fast \
  > "$repo/private/ag1/$name.log" 2>&1
echo $? > "$repo/private/ag1/$name.exit"
