#!/usr/bin/env bash
# archive.sh BEFORE AFTER -- CG05-ELABORATOR-CALL-FOOTPRINTS: write the report's
# artifacts from two measure.sh runs (private/cg05ef/NAME*). Nothing piped;
# each step's exit is checked.
set -eu
before="$1"
after="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
p="$repo/private/cg05ef"
a="$here/../artifacts"
mkdir -p "$a"
python3 "$here/compare.py" "$p/$before" "$p/$after" --json "$a/corpus-comparison.json" > /dev/null
python3 "$here/changed.py" "$p/$before" "$p/$after" --json "$a/corpus-changed.json" > /dev/null
python3 "$here/compare.py" "$p/$before-system" "$p/$after-system" --json "$a/system-comparison.json" > /dev/null
python3 "$here/identity.py" "$before" "$after" > "$a/identity.json"
for run in "$before" "$after"; do
  python3 "$here/census.py" "$p/$run-parity.jsonl" --json "$a/census-$run.json" --rows "$p/$run-rows.jsonl" > /dev/null
  python3 "$here/families.py" "$p/$run-rows.jsonl" --json "$a/families-$run.json" > /dev/null
  python3 "$here/predict.py" "$p/$run" "$p/$run-rows.jsonl" --json "$a/predict-$run.json" > /dev/null
done
