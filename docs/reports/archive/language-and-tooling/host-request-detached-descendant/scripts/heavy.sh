#!/usr/bin/env bash
# heavy.sh LABEL COMMAND... -- HOST-REQUEST-READ-BY-DETACHED-DESCENDANT (from HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING):
# run one heavy step from the worktree root under the shared heavy-step lock,
# its output to private/hdd/LABEL.log and its exit to private/hdd/LABEL.exit.
# Nothing is piped.
set -u
label="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=$(ls -d /tmp/claude-1000/*/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock 2>/dev/null | head -1)
: "${lock:=$repo/private/hdd/heavy.lock}"
mkdir -p "$repo/private/hdd" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" "$@" > "private/hdd/$label.log" 2>&1
rc=$?
echo "$rc" > "private/hdd/$label.exit"
exit "$rc"
