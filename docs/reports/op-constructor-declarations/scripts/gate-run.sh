#!/usr/bin/env bash
# gate-run.sh NAME -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES: take the shared
# heavy lock once and run the whole gate under it (gate-whole.sh NAME), then
# clean the dev profile under the lock again.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
flock "$lock" bash "$here/gate-whole.sh" "$name"
echo "gate-whole exit $?"
cd "$repo" || exit 2
flock "$lock" cargo clean --profile dev > "$repo/private/opctor/gate-$name/07z-clean.log" 2>&1
echo $? > "$repo/private/opctor/gate-$name/07z-clean.exit"
grep -H . "$repo/private/opctor/gate-$name"/*.exit
