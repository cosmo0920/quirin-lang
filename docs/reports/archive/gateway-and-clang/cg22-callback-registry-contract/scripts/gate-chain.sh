#!/usr/bin/env bash
# gate-chain.sh NAME -- CG22-CALLBACK-REGISTRY-CONTRACT: the standing gate's
# code part (gate.sh NAME code, the suite logs under tests-NAME), then
# `cargo clean --profile dev` under the shared lock (07z-clean, the brief's
# step after the test step), then the provider gate (provider-gate.sh NAME).
# Exits to files; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
SUITE_NAME="$name" bash "$here/gate.sh" "$name" code
cd "$repo" || exit 2
CARGO_BUILD_JOBS=4 flock "$lock" cargo clean --profile dev > "private/cg22r/gate-$name/07z-clean.log" 2>&1
echo $? > "private/cg22r/gate-$name/07z-clean.exit"
bash "$here/provider-gate.sh" "$name"
grep -H . "private/cg22r/gate-$name"/*.exit "private/cg22r/provider-gate-$name"/*.exit
