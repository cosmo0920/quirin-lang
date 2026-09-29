#!/bin/sh
# compare_all.sh BEFORE AFTER -- CG06-VALUE-CONNECTIVES-AND-CONDITIONAL: the
# corpus, system and tail-pause slice comparisons of two measure.sh runs, into
# artifacts/, then the census and absorbed-follow-up re-measurement of AFTER.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$here/../artifacts"
runs="$repo/private/cg06"
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
before="$1"; after="$2"
python3 "$here/compare.py" "$runs/$before" "$runs/$after" --json "$out/corpus-comparison.json" > /dev/null
python3 "$here/compare.py" "$runs/$before-system" "$runs/$after-system" --json "$out/system-comparison.json" > /dev/null
python3 "$here/compare.py" "$slice/cg06-$before" "$slice/cg06-$after" --json "$out/slice-comparison.json" > /dev/null
python3 "$here/changed.py" "$runs/$before" "$runs/$after" --json "$out/corpus-changed.json" > "$runs/$after-changed.txt"
python3 "$here/parity_census.py" "$runs/$after-parity.jsonl" --json "$out/parity-census.json" --examples 5 > "$runs/$after-census.txt"
python3 "$here/absorbed.py" "$runs/$after" "$runs/$after-parity.jsonl" --json "$out/absorbed.json" > /dev/null
python3 "$here/rederive.py" "$runs/$after" --json "$out/rederive-after.json" > "$runs/$after-rederive.txt"
for f in corpus system slice; do
  python3 - "$out/$f-comparison.json" "$f" <<'EOF'
import json, sys
d = json.load(open(sys.argv[1]))
print(sys.argv[2], d["totals"]["after"], d["move_counts"], d["same_status_functions_with_changed_diagnostics"])
for m in d["moves"]:
    print("  ", m["before"], "->", m["after"], m["unit"], m["function"])
print("  ", d["leaf_delta"])
EOF
done
