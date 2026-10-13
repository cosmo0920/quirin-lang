#!/usr/bin/env bash
# build.sh -- SUP03-BOUNDS-TALLY-ON-INTEGRATED-MAIN: in this checkout, run the
# submodule step, then build the release quirin-audit (the binary
# run_pinned_audit.py runs, target/release/quirin-audit) under the shared heavy
# lock, and record HEAD, the tree's cleanliness and the binary's digest.
# Output: private/sup03/ (worktree-private). Exits to files; nothing piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/sup03"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
git submodule update --init --recursive > "$out/submodule.log" 2>&1
echo $? > "$out/submodule.exit"
git rev-parse HEAD > "$out/build.head"
git status --porcelain --untracked-files=no > "$out/build.dirty"
flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit > "$out/build.log" 2>&1
echo $? > "$out/build.exit"
[ "$(cat "$out/build.exit")" = 0 ] || exit 1
sha256sum target/release/quirin-audit > "$out/build.binary.sha256"
cat "$out/build.binary.sha256"
