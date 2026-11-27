#!/usr/bin/env bash
# build.sh NAME TARGET_DIR -- AG1: the census-feature release quirin-audit of
# the working tree into private/ag1/TARGET_DIR, in a `build` gate slot.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
target="$repo/private/ag1/$2"
cd "$repo" || exit 2
CARGO_TARGET_DIR="$target" python3 scripts/gate_slot.py build \
  --receipt "$repo/private/ag1/$name.receipt.json" -- \
  cargo build --release -p qcore-audit --bin quirin-audit \
  --features qcore-c-gateway/indirect-census \
  > "$repo/private/ag1/$name.log" 2>&1
echo $? > "$repo/private/ag1/$name.exit"
