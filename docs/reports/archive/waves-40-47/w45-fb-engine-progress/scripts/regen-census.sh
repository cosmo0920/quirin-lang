#!/usr/bin/env bash
# Rebuild the published census from a prepared attempt and its slice run.
# usage: regen-census.sh ATTEMPT SLICE_RUN   (e.g. attempt-1 slice-37e1b82d)
# The inventory is written privately first, then copied; census.json is joined
# from it, the hand classification and the slice report (fail-closed).
set -u
attempt="$1"; run="$2"
here="$(cd "$(dirname "$0")" && pwd)"
report="$(cd "$here/.." && pwd)"
root=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-engine-progress/$attempt
mkdir -p "$root/census"
python3 "$here/loop_inventory.py" "$root" --out "$root/census/loop-inventory.json" --work "$root/census/work" || exit 1
cp "$root/census/loop-inventory.json" "$report/census/loop-inventory.json"
python3 "$here/census.py" "$root/census/loop-inventory.json" "$root/$run/resource-diagnostics.json" \
  --out "$report/census/census.json"
