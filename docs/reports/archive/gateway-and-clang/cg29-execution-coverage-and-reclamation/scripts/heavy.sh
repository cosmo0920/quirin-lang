#!/usr/bin/env bash
# heavy.sh -- the AFTER (after.sh), then the standing gate (gate.sh, steps
# 01-09) into private/cg29/gate, then `cargo clean --profile dev`. Run under
# the shared heavy lock.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
bash "$here/after.sh"
bash "$here/gate.sh" "$repo/private/cg29/gate" 01 02 03 05 06 07 08 09
cd "$repo" && cargo clean --profile dev > "$repo/private/cg29/clean-dev.log" 2>&1
echo $? > "$repo/private/cg29/clean-dev.exit"
