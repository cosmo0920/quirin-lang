#!/usr/bin/env bash
# compare_all.sh -- write every comparison of this card's runs into
# artifacts/compare/ (compare.py) and artifacts/compare/*-files.txt
# (before_after.py). Exits are printed.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
out="$here/../artifacts/compare"
mkdir -p "$out"
base=docs/reports/w47-condvar-join-publication/integration/artifacts
tail=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
run() { python3 "$here/compare.py" "$1" "$2" --json "$out/$3.json" > /dev/null; echo "$3 $?"; }
run "$base/corpus" private/rh/before-corpus/corpus corpus-baseline-before
run "$base/corpus" private/rh/after/corpus corpus-baseline-after
run "$base/system" private/rh/before/system system-baseline-before
run private/rh/before/system private/rh/after/system system-before-after
run "$base/slice" "$tail/rh-before" slice-baseline-before
run "$tail/rh-before" "$tail/rh-after" slice-before-after
python3 "$here/before_after.py" private/rh/before-corpus/corpus private/rh/after/corpus > "$out/corpus-files.txt"
python3 "$here/before_after.py" private/rh/before/system private/rh/after/system > "$out/system-files.txt"
python3 "$here/before_after.py" "$tail/rh-before" "$tail/rh-after" > "$out/slice-files.txt"
if [ -d private/rh/if-before ] && [ -d private/rh/if-after ]; then
  run private/rh/if-before private/rh/if-after instance-flow-probe-before-after
fi
