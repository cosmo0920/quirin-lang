#!/bin/sh
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-af93c329e85457c34
cd "$root" || exit 2
overall=0
for name in "$@"; do
  python3 docs/reports/w44-audit-parallel-units/scripts/compare_runs.py private/runs-current/base "private/runs-current/$name" --mask-preparation > "private/compare/current-base-vs-$name.json"
  status=$?
  echo "compare_runs current base vs $name (preparation figures masked) exit=$status"
  [ $status -eq 0 ] || overall=1
  grep -v '^preparation:' private/runs-current/base/audit.log > private/compare/current-base.log.nopre
  grep -v '^preparation:' "private/runs-current/$name/audit.log" > "private/compare/current-$name.log.nopre"
  cmp private/compare/current-base.log.nopre "private/compare/current-$name.log.nopre"
  echo "cmp audit.log without preparation line base vs $name exit=$?"
  diff private/runs-current/base/audit.log "private/runs-current/$name/audit.log"
  echo "raw diff exit=$?"
done
echo "overall=$overall"
exit $overall
