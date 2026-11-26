#!/bin/sh
# compare.sh -- totals of both runs, every moved row and every changed row.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ae8e8b56de03e13f5
S=$W/docs/reports/call-with-deref-actual/scripts
R=$W/private/cwd
for n in base2 tip; do
  echo "$n exit=$(cat $R/run-$n/exit) build=$(cat $R/run-$n/build.exit) checkers=$(cat $R/run-$n/checkers.exit) $(cat $R/run-$n/started) $(cat $R/run-$n/finished)"
  python3 "$S/totals.py" "$R/run-$n/resource-diagnostics.json"
done
python3 "$S/moved.py" "$R/run-base2/resource-diagnostics.json" "$R/run-tip/resource-diagnostics.json" > "$R/moved.jsonl"
python3 "$S/changed_rows.py" "$R/run-base2/resource-diagnostics.json" "$R/run-tip/resource-diagnostics.json" > "$R/changed.txt"
cat "$R/moved.jsonl"
cat "$R/changed.txt"
