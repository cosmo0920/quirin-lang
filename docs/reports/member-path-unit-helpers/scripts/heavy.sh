#!/usr/bin/env bash
# heavy.sh LOG CMD... -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS (adapted from OP-CALLBACK-MEMBER-PATH-RELEASE) (adapted from SUPPLIED-OWNED-RETURN-CODE-HANDOFF) (adapted from RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION): run CMD from the
# worktree root under the shared exclusive heavy lock, output to LOG, exit to
# LOG.exit.
set -u
log="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo" || exit 2
mkdir -p "$(dirname "$log")"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-6}" CARGO_INCREMENTAL=0
flock "$lock" "$@" > "$log" 2>&1
status=$?
echo "$status" > "$log.exit"
exit "$status"
