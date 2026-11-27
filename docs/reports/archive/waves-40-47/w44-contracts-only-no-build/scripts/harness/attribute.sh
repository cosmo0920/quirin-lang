#!/bin/sh
# Attribute every no-build run of private/measure/RUN against its configured run.
# usage: attribute.sh RUN
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-aa974c0228e7d5c45
prep=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-hungry-thompson-1740da/4e877459-f647-4ada-9799-501505fa7da8/scratchpad/prep
fb=/home/cosmo/Documents/GitHub/fluent-bit
cd "$root" || exit 2
run="$root/private/measure/${1:?usage: attribute.sh RUN}"
for name in bare src-includes root; do
  if [ -f "$run/$name/report.json" ]; then
    python3 docs/reports/w44-contracts-only-no-build/scripts/compare.py \
      "$run/configured/contracts-only.json" "$run/$name/report.json" "$fb" "$prep" \
      "$run/attribution-$name.json" > "$run/attribution-$name.log" 2>&1
    echo $? > "$run/attribution-$name.exit"
    echo "$name exit=$(cat "$run/attribution-$name.exit")"
  fi
done
