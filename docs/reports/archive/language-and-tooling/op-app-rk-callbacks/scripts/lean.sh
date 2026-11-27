#!/usr/bin/env bash
# lean.sh [TARGET...] -- OP-APP-RK-CALLBACKS-VIA-POLL-CB (adapted from
# OP-CONSTRUCTOR-DECLARATIONS): build Lean targets of this worktree's proof
# project (default QCore.AppRkCallbacks), from inside proof/ so elan reads
# proof/lean-toolchain, under the shared heavy lock. The log goes to
# private/oprk/logs/lean.log and the exit to lean.exit.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$repo/private/oprk/logs"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$logs" || exit 2
cd "$repo/proof" || exit 2
if [ "$#" -eq 0 ]; then
  set -- QCore.AppRkCallbacks
fi
flock "$lock" lake build "$@" > "$logs/lean.log" 2>&1
echo $? > "$logs/lean.exit"
cat "$logs/lean.exit"
