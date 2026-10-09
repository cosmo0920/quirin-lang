#!/usr/bin/env bash
# census.sh RUN -- RDHTTP-HEADER-SUMMARY-GAP: census.py over RUN's unvalidated
# units (private/rhsg/units-RUN.txt) under the shared heavy lock, into
# private/rhsg/census-RUN.{txt,exit}.
set -u
run="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
flock "$lock" python3 "$here/census.py" "$repo/private/rhsg/units-$run.txt" "$repo/private/rhsg/census-$run.txt" > "$repo/private/rhsg/census-$run.log" 2>&1
echo $? > "$repo/private/rhsg/census-$run.exit"
cat "$repo/private/rhsg/census-$run.exit"
