#!/bin/sh
# identity + pairwise compare vs j1 + raw cmp of audit.log, for each named run
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-af93c329e85457c34
cd "$root" || exit 2
overall=0
for name in "$@"; do
  echo "=== $name"
  private/identity.sh "$name" || overall=1
  python3 docs/reports/w44-audit-parallel-units/scripts/compare_runs.py private/runs/j1 "private/runs/$name" > "private/compare/j1-vs-$name.json"
  status=$?
  echo "compare_runs j1 vs $name exit=$status"
  [ $status -eq 0 ] || overall=1
  cmp private/runs/j1/audit.log "private/runs/$name/audit.log"
  status=$?
  echo "cmp audit.log j1 vs $name exit=$status"
  [ $status -eq 0 ] || overall=1
done
echo "overall=$overall"
exit $overall
