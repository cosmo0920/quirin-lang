#!/usr/bin/env bash
# tests-step.sh NAME SUITE -- CG22-CALLBACK-REGISTRY-CONTRACT: gate.sh's step 04
# alone (the suite through run_test_binaries.py under the shared lock, logs in
# private/nsp/tests-SUITE, output and exit to private/nsp/gate-NAME/04-tests.*),
# then `cargo clean --profile dev` (07z). For a gate whose step 04 never ran
# (gate g3: a stale log directory of a stopped attempt refused it before any
# test). Nothing piped.
set -u
name="$1"; suite="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/nsp/gate-$name"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
git rev-parse HEAD > "$out/04-tests.head"
flock "$lock" python3 scripts/run_test_binaries.py --log-dir "$repo/private/nsp/tests-$suite" > "$out/04-tests.log" 2>&1
echo $? > "$out/04-tests.exit"
flock "$lock" cargo clean --profile dev > "$out/07z-clean.log" 2>&1
echo $? > "$out/07z-clean.exit"
cat "$out/04-tests.exit"
