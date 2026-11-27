#!/usr/bin/env bash
# heavy.sh LOG COMMAND... -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2: run
# one heavy command (a build, a test run) under the shared heavy lock with
# CARGO_BUILD_JOBS=4, its output in LOG and its exit in LOG.exit. Nothing piped.
set -u
log="$1"; shift
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" "$@" > "$log" 2>&1
echo $? > "$log.exit"
cat "$log.exit"
