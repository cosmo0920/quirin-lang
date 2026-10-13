#!/usr/bin/env bash
# providers-check.sh NAME -- QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN: a compile
# check of the provider workspace, whose conformance crate depends on
# qcore-verifier-host by path. Not the provider gate (providers/ is unchanged,
# so that gate is not due); it confirms the changed crate's public surface
# still compiles there. Under the shared heavy lock; exit to a file.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/qpdf/gate-$1"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo check --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline > "$out/providers-check.log" 2>&1
echo $? > "$out/providers-check.exit"
