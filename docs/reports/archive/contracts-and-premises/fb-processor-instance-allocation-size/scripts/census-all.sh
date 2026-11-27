#!/usr/bin/env bash
# census-all.sh OUT -- FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE: census.py over
# the four configurations, under the shared heavy lock, each exit to a file.
set -u
out="$1"
here="$(cd "$(dirname "$0")" && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
for config in corpus-66910c10 system-046accf6 pins-046accf6 slice-34155955; do
  flock "$lock" python3 "$here/census.py" "$config" "$out" --jobs 4 > "$out/$config.log" 2>&1
  echo $? > "$out/$config.exit"
done
