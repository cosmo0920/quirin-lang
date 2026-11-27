#!/usr/bin/env bash
# provider-tests-isolated.sh NAME -- CG22-CALLBACK-REGISTRY-CONTRACT: the
# provider gate's `cargo test` step again, alone under the shared lock, for a
# load-sensitive failure (constraint 16: the ETXTBSY race of a package's
# provider binary) verified in isolation, never weakened. Output and exit to
# private/cg22r/provider-gate-NAME/p2-test-isolated.*; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/cg22r/provider-gate-$name"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline > "$out/p2-test-isolated.log" 2>&1
echo $? > "$out/p2-test-isolated.exit"
cat "$out/p2-test-isolated.exit"
