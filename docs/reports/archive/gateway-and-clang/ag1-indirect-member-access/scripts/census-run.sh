#!/usr/bin/env bash
# census-run.sh NAME SELECTION [JOBS] [TARGET_DIR] -- AG1-INDIRECT-MEMBER-ACCESS:
# the unit-scoped analyzer census (run_current_census.py) with the
# indirect-census build at private/ag1/TARGET_DIR (default target-census, the
# base), in one `audit` gate slot. Exits to files.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
selection="$2"
jobs="${3:-4}"
target="${4:-target-census}"
out="$repo/private/ag1"
python3 "$repo/scripts/gate_slot.py" audit --memory-budget 7G \
  --receipt "$out/$name.receipt.json" -- \
  python3 "$here/run_current_census.py" "$out/$name" \
  "$out/$target/release/quirin-audit" "$selection" "$jobs" \
  > "$out/$name.console.log" 2>&1
echo $? > "$out/$name.exit"
