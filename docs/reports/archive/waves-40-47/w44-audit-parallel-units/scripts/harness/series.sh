#!/bin/sh
# Runs the pinned arbiter series one after another; stops at the first failure.
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-af93c329e85457c34
for spec in "$@"; do
  name=${spec%%:*}
  jobs=${spec##*:}
  "$root/private/timed.sh" "$name" --jobs "$jobs" || { echo "series stopped at $name"; exit 1; }
done
echo "series done"
