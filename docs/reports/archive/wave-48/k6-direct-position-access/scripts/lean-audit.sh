#!/usr/bin/env bash
# lean-audit.sh NAME AUDIT.lean -- K6: elaborate one proof/Audit*.lean file
# (its `#print axioms` lines) in a `lean` gate slot; log and exit under private/k6/.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
audit="$2"
cd "$repo/proof" || exit 2
python3 "$repo/scripts/gate_slot.py" lean --receipt "$repo/private/k6/$name.receipt.json" -- \
  lake env lean "$audit" > "$repo/private/k6/$name.log" 2>&1
echo $? > "$repo/private/k6/$name.exit"
