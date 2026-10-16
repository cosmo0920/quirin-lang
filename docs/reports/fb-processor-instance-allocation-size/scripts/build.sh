#!/usr/bin/env bash
# build.sh OUT -- FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE: this worktree's
# release quirin-audit under the shared heavy lock; log and exit to OUT.
set -u
out="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 flock "$lock" \
  cargo build --release -p qcore-audit --bin quirin-audit > "$out/build.log" 2>&1
echo $? > "$out/build.exit"
