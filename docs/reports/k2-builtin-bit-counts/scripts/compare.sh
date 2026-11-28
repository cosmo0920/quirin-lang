#!/bin/sh
# compare.sh -- totals, moved rows and changed rows of the opt-in runs.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ae71e0b9f85fdedfd
S=$W/docs/reports/k2-builtin-bit-counts/scripts
M=$W/private/k2bc/m
for n in base tip; do echo "$n exit=$(cat $M/run-$n/exit) slot=$(cat $M/$n.optin.slot.exit) $(cat $M/run-$n/started $M/run-$n/finished | tr '\n' ' ')"; done
python3 "$S/totals.py" "$M/run-base/resource-diagnostics.json"
python3 "$S/totals.py" "$M/run-tip/resource-diagnostics.json"
python3 "$S/moved.py" "$M/run-base/resource-diagnostics.json" "$M/run-tip/resource-diagnostics.json" > "$M/moved.jsonl"
cut -c1-2000 "$M/moved.jsonl"
python3 "$S/changed_rows.py" "$M/run-base/resource-diagnostics.json" "$M/run-tip/resource-diagnostics.json" > "$M/changed.txt"
cut -c1-500 "$M/changed.txt"
