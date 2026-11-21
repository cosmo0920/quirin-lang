#!/bin/sh
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a43d32eda6d150c67
S=$W/docs/reports/cg10-owned-out-parameter-member-slot/scripts
O=$W/private/oom
cd "$O" || exit 2
cat run-tip2/exit run-tip2/started run-tip2/finished
python3 $S/totals.py run-base/resource-diagnostics.json > totals-base.json
python3 $S/totals.py run-tip2/resource-diagnostics.json > totals-tip2.json
cat totals-base.json totals-tip2.json
python3 $S/moved.py run-base/resource-diagnostics.json run-tip2/resource-diagnostics.json > moved.jsonl
python3 $S/changed_rows.py run-base/resource-diagnostics.json run-tip2/resource-diagnostics.json > changed.txt
wc -l moved.jsonl changed.txt
grep -hE "Safe row|declarations consulted|validated|refuted" run-base/console.log run-tip2/console.log | head -12
