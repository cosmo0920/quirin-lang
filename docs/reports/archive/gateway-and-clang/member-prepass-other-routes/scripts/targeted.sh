#!/usr/bin/env bash
# targeted.sh NAME -- MEMBER-PREPASS-OTHER-ROUTES: the targeted check after
# a rebase onto a main whose landed code is disjoint from this card's
# (landing-race policy): cargo check, the card's and the checkpoint tests,
# clippy, the Lean audits (the merged proof/QCore.lean) and fmt, each under
# the shared heavy lock, exits to private/mpor/targeted-NAME/.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/mpor/targeted-$name"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
git rev-parse HEAD > "$out/head"
step() { local label="$1"; shift; flock "$lock" "$@" > "$out/$label.log" 2>&1; echo $? > "$out/$label.exit"; }
step 00-check cargo check --workspace --all-targets
step 01-fmt cargo fmt --all -- --check
step 02-clippy cargo clippy --locked --all-targets -- -D warnings
step 03-tests cargo test --no-fail-fast -p qcore-audit --test member_prepass_routes --test checkpoint_cli --test dump_checkpoint_cli
step 04-lean python3 scripts/check_lean_audits.py
step 05-clean cargo clean --profile dev
grep -H . "$out"/*.exit
