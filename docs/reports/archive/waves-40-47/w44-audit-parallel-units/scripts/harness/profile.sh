#!/bin/sh
# usage: profile.sh BINARY NAME -> private/profile/NAME
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-af93c329e85457c34
mkdir -p "$root/private/profile"
python3 "$root/docs/reports/w44-audit-parallel-units/scripts/profile_run.py" "$1" "$root/private/profile/$2" \
  > "$root/private/logs/profile-$2.out" 2>&1
status=$?
echo "exit=$status" >> "$root/private/logs/profile-$2.out"
exit $status
