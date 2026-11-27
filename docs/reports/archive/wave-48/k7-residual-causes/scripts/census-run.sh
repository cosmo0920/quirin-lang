#!/usr/bin/env bash
# census-run.sh NAME SELECTION [JOBS] [TARGET_DIR] -- K7-RESIDUAL-CAUSES
# (AG1's census-run.sh, paths moved): the unit-scoped analyzer run
# (run_current_census.py) with the build at private/k7/TARGET_DIR (default
# target-census, the base), in one `audit` gate slot. Exits to files.
# K7_REPORT_ONLY=1 in the environment switches the census streams off.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
selection="$2"
jobs="${3:-4}"
target="${4:-target-census}"
out="$repo/private/k7"
python3 "$repo/scripts/gate_slot.py" audit --memory-budget 7G \
  --receipt "$out/$name.receipt.json" -- \
  python3 "$here/run_current_census.py" "$out/$name" \
  "$out/$target/release/quirin-audit" "$selection" "$jobs" \
  > "$out/$name.console.log" 2>&1
echo $? > "$out/$name.exit"
