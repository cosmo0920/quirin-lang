#!/usr/bin/env bash
# heavy.sh LOG COMMAND... -- NULL-CANDIDATE-TRIAGE (from NULL-SAFETY-PROPERTY): run one heavy step (a build
# or a test binary) from the repository root under the shared heavy lock, with
# CARGO_BUILD_JOBS=4 and CARGO_INCREMENTAL=0; output to LOG, exit to LOG.exit.
set -u
log="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" "$@" > "$log" 2>&1
echo $? > "$log.exit"
cat "$log.exit"
