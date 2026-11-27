#!/usr/bin/env bash
# build.sh NAME -- K6: build the release quirin-audit binary in a `build` gate
# slot, target under private/k6/target-NAME, and copy it to private/k6/bin/.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
target="$repo/private/k6/target-$name"
mkdir -p "$repo/private/k6/bin"
cd "$repo" || exit 2
CARGO_TARGET_DIR="$target" python3 scripts/gate_slot.py build \
  --receipt "$repo/private/k6/build-$name.receipt.json" -- \
  cargo build --release -p qcore-audit \
  > "$repo/private/k6/build-$name.log" 2>&1
status=$?
echo $status > "$repo/private/k6/build-$name.exit"
if [ $status -eq 0 ]; then
  cp "$target/release/quirin-audit" "$repo/private/k6/bin/quirin-audit-$name"
  sha256sum "$repo/private/k6/bin/quirin-audit-$name" > "$repo/private/k6/bin/quirin-audit-$name.sha256"
  git -C "$repo" rev-parse HEAD > "$repo/private/k6/bin/quirin-audit-$name.rev"
fi
exit $status
