#!/usr/bin/env bash
# build.sh NAME TARGET_DIR [plain] -- K7: the release quirin-audit of the
# working tree into private/k7/TARGET_DIR, in a `build` gate slot (AG1's
# build.sh, paths moved). Without `plain` it is the census-feature build.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
target="$repo/private/k7/$2"
features=(--features qcore-c-gateway/indirect-census)
if [ "${3:-}" = "plain" ]; then features=(); fi
mkdir -p "$repo/private/k7"
cd "$repo" || exit 2
CARGO_TARGET_DIR="$target" python3 scripts/gate_slot.py build \
  --receipt "$repo/private/k7/$name.receipt.json" -- \
  cargo build --release -p qcore-audit --bin quirin-audit "${features[@]}" \
  > "$repo/private/k7/$name.log" 2>&1
echo $? > "$repo/private/k7/$name.exit"
