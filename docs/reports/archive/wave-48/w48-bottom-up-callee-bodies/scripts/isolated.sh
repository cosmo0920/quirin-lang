#!/usr/bin/env bash
# isolated.sh NAME LABEL PACKAGE TEST -- W48-BOTTOM-UP-CALLEE-BODIES: one test
# binary re-run alone (constraint 16, a load- or environment-sensitive
# failure verified in isolation, nothing weakened), output and exit to
# private/w48bu/gate-NAME/LABEL.*, under the shared heavy lock.
set -u
name="$1"; label="$2"; package="$3"; test="$4"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/w48bu/gate-$name"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo test -p "$package" --test "$test" > "$out/$label.log" 2>&1
echo $? > "$out/$label.exit"
flock "$lock" cargo clean --profile dev > "$out/$label-clean.log" 2>&1
