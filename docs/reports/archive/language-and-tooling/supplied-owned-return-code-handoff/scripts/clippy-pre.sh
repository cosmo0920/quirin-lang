#!/usr/bin/env bash
# clippy-pre.sh NAME -- SUPPLIED-OWNED-RETURN-CODE-HANDOFF: fmt check and
# clippy (as the gate runs them) under the shared heavy lock before the gate,
# output and exits to private/sorch/pre-NAME.*. Not gate evidence.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0
cargo fmt --all -- --check > "private/sorch/pre-$name-fmt.log" 2>&1
echo $? > "private/sorch/pre-$name-fmt.exit"
flock "$lock" cargo clippy --locked --all-targets -- -D warnings > "private/sorch/pre-$name-clippy.log" 2>&1
echo $? > "private/sorch/pre-$name-clippy.exit"
