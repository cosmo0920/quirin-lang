#!/usr/bin/env bash
# lean-audit.sh NAME AUDIT.lean -- K1: elaborate one proof/Audit*.lean file
# (its `#print axioms` lines) in a `lean` gate slot; log and exit under private/k1/.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
audit="$2"
cd "$repo/proof" || exit 2
python3 "$repo/scripts/gate_slot.py" lean --receipt "$repo/private/k1/$name.receipt.json" -- \
  lake env lean "$audit" > "$repo/private/k1/$name.log" 2>&1
echo $? > "$repo/private/k1/$name.exit"
