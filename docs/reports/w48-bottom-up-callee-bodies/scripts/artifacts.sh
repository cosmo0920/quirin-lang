#!/usr/bin/env bash
# artifacts.sh BEFORE AFTER -- W48-BOTTOM-UP-CALLEE-BODIES: the plan, family,
# held-call and ranking analyses of two measure.sh runs (private/w48bu/NAME*)
# into artifacts/after/ and artifacts/ranking/, with the attribution of every
# changed row on the three routes.
set -u
before="$1"
after="$2"
here="$(cd "$(dirname "$0")" && pwd)"
report="$(cd "$here/.." && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
slices=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
out="$report/artifacts/after"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
bash "$here/rank_all.sh" "$after" after
python3 "$here/bottom_up_plan.py" "private/w48bu/$after" "$out/plan-after.json" --top 30 > "$out/plan-after.txt"
python3 "$here/held_calls.py" "private/w48bu/$after" "private/w48bu/$after-parity.jsonl" "$out/held-calls-after.json" > "$out/held-calls-after.txt"
python3 "$here/family_leverage.py" "private/w48bu/$after" "$out/family-corpus-after.json" --top 25 > "$out/family-corpus-after.txt"
python3 "$here/attribute.py" "private/w48bu/$before" "private/w48bu/$after" "$out/attribution-corpus.json" > "$out/attribution-corpus.txt"
python3 "$here/attribute.py" "private/w48bu/$before-system" "private/w48bu/$after-system" "$out/attribution-system.json" > "$out/attribution-system.txt"
python3 "$here/attribute.py" "$slices/w48bu-$before" "$slices/w48bu-$after" "$out/attribution-slice.json" > "$out/attribution-slice.txt"
python3 "$here/totals.py" "private/w48bu/$before" "private/w48bu/$after" "private/w48bu/$before-system" \
  "private/w48bu/$after-system" "$slices/w48bu-$before" "$slices/w48bu-$after" > "$out/totals.txt"
cp "private/w48bu/$after.binary.sha256" "$out/binary.sha256"
gzip -f "$out/plan-after.json" "$report/artifacts/ranking/corpus-after.json"
