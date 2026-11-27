#!/usr/bin/env bash
# build.sh NAME TARGET_DIR -- K2: the release quirin-audit of the working tree
# into private/k2/TARGET_DIR, in a `build` gate slot (K7's build.sh, plain
# build: the parity stream is env-gated, no census feature is needed).
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
target="$repo/private/k2/$2"
mkdir -p "$repo/private/k2"
cd "$repo" || exit 2
CARGO_TARGET_DIR="$target" python3 scripts/gate_slot.py build \
  --receipt "$repo/private/k2/$name.receipt.json" -- \
  cargo build --release -p qcore-audit --bin quirin-audit \
  > "$repo/private/k2/$name.log" 2>&1
echo $? > "$repo/private/k2/$name.exit"
