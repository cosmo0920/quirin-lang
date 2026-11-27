#!/usr/bin/env bash
# DECLARED-ASSUMPTION-BODY-CHECKS: BEFORE (main d8a1a2a4e) and AFTER (a2) on the
# rebased base, three routes each, then the identity comparison.
set -u
repo=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-aec57edb5e625291d
s="$repo/docs/reports/declared-assumption-body-checks/scripts"
bash "$s/measure-binary.sh" before2 "$repo/private/dabc/bin/before2" > "$repo/private/dabc/measure-before2.out" 2>&1
bash "$s/measure-binary.sh" a2 "$repo/private/dabc/bin/a2" > "$repo/private/dabc/measure-a2.out" 2>&1
bash "$s/identity.sh" before2 a2 > "$repo/private/dabc/identity-a2.out" 2>&1
echo done > "$repo/private/dabc/measure2.done"
