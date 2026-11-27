#!/bin/sh
# usage: timed.sh NAME [run_audit.py extra args...]
# Runs the arbiter into private/runs/slot under /usr/bin/time -v, then renames
# it to private/runs/NAME, so every run's audit.log names the same output path
# and two runs' logs can be compared raw with cmp.
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-af93c329e85457c34
name=$1
shift
commit=$(cat "$root/private/commit.txt")
if [ -e "$root/private/runs/slot" ] || [ -e "$root/private/runs/$name" ]; then
  echo "slot or $name exists" >&2
  exit 2
fi
uptime > "$root/private/time/$name.load"
/usr/bin/time -v -o "$root/private/time/$name.time" \
  python3 "$root/docs/reports/w43-return-expression/scripts/run_audit.py" \
  "$root/private/runs/slot" "$commit" "$@" > "$root/private/logs/run-$name.out" 2>&1
status=$?
echo "exit=$status" >> "$root/private/logs/run-$name.out"
mv "$root/private/runs/slot" "$root/private/runs/$name"
exit $status
