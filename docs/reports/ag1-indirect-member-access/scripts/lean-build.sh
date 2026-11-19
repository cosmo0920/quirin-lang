#!/usr/bin/env bash
# lean-build.sh NAME TARGET... -- AG1: `lake build TARGET...` in proof/ in a
# `lean` gate slot; log and exit under private/ag1/.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
shift
cd "$repo/proof" || exit 2
python3 "$repo/scripts/gate_slot.py" lean --receipt "$repo/private/ag1/$name.receipt.json" -- \
  lake build "$@" > "$repo/private/ag1/$name.log" 2>&1
echo $? > "$repo/private/ag1/$name.exit"
