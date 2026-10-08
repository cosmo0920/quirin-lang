#!/usr/bin/env bash
# targeted.sh NAME PACKAGE [CARGO-TEST-ARGS...] -- SLOT-MOVE-AND-INTERIOR-BORROW:
# one cargo test invocation of PACKAGE under the shared heavy lock, its log and
# exit in private/rrrd/NAME.{log,exit}. Nothing is piped.
set -u
name="$1"; package="$2"; shift 2
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/rrrd"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo test -p "$package" "$@" > "$out/$name.log" 2>&1
echo $? > "$out/$name.exit"
cat "$out/$name.exit"
