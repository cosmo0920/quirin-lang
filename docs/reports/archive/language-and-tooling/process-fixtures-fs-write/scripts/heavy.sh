#!/usr/bin/env bash
# heavy.sh LABEL COMMAND... -- PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE
# (from HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING's): run one heavy step
# from the worktree root under the shared heavy-step lock, its output to
# private/pfw/LABEL.log and its exit to private/pfw/LABEL.exit. Nothing is
# piped.
set -u
label="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$repo/private/pfw" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" "$@" > "private/pfw/$label.log" 2>&1
rc=$?
echo "$rc" > "private/pfw/$label.exit"
exit "$rc"
