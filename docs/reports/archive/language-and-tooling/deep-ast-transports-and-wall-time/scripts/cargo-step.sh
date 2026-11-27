#!/usr/bin/env bash
# cargo-step.sh NAME CARGO-ARGS... -- DEEP-AST-TRANSPORTS-AND-WALL-TIME (from
# GATEWAY-ANALYSIS-PATH-OPTIMIZATION's scripts/cargo-step.sh): one cargo
# command (check, a targeted test) under the shared heavy lock with
# CARGO_BUILD_JOBS=4 and CARGO_INCREMENTAL=0; output to private/dat/NAME.log,
# exit to private/dat/NAME.exit; nothing piped.
set -u
name="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/dat"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo "$@" > "$out/$name.log" 2>&1
rc=$?
echo "$rc" > "$out/$name.exit"
echo "exit $rc"
exit "$rc"
