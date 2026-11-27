#!/usr/bin/env bash
# lean-build.sh NAME TARGET... -- K1: `lake build TARGET...` in proof/ in a
# `lean` gate slot; log and exit under private/k1/.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
shift
cd "$repo/proof" || exit 2
python3 "$repo/scripts/gate_slot.py" lean --receipt "$repo/private/k1/$name.receipt.json" -- \
  lake build "$@" > "$repo/private/k1/$name.log" 2>&1
echo $? > "$repo/private/k1/$name.exit"
