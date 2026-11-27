#!/usr/bin/env bash
# check.sh NAME -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES: `cargo check
# --workspace --all-targets` under the shared heavy lock, log and exit into
# private/cg22op/check-NAME.{log,exit}. Nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg22op"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo check --workspace --all-targets > "$out/check-$name.log" 2>&1
echo $? > "$out/check-$name.exit"
cat "$out/check-$name.exit"
