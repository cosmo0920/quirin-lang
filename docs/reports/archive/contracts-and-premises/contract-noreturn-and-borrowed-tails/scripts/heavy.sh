#!/usr/bin/env bash
# heavy.sh LOG_NAME COMMAND... -- run one heavy step from this worktree under
# the shared heavy lock with the round-10 host limits (CARGO_BUILD_JOBS=4,
# CARGO_INCREMENTAL=0), output to private/cnbt/LOG_NAME.log and its exit to
# private/cnbt/LOG_NAME.exit (never piped). Prints the exit.
set -u
name="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$repo/private/cnbt"
cd "$repo" || exit 2
CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 flock "$lock" "$@" > "$repo/private/cnbt/$name.log" 2>&1
echo $? > "$repo/private/cnbt/$name.exit"
echo "$name exit $(cat "$repo/private/cnbt/$name.exit")"
