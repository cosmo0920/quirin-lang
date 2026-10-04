#!/usr/bin/env bash
# lean-module.sh -- CHUNKIO-META-LEN-BOUNDS-CONTRACTS: build the card's Lean
# module (and what it imports) in proof/, output and exit to
# private/chunkio-meta-len-bounds-contracts/lean-module.{log,exit}.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/chunkio-meta-len-bounds-contracts"
mkdir -p "$out" || exit 2
cd "$repo/proof" || exit 2
timeout 1800 lake build QCore.BoundsPinHold > "$out/lean-module.log" 2>&1
echo $? > "$out/lean-module.exit"
cat "$out/lean-module.exit"
