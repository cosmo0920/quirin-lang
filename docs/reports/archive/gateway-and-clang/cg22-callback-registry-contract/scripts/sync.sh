#!/usr/bin/env bash
# sync.sh NAME -- CG22-CALLBACK-REGISTRY-CONTRACT (adapted from REFCOUNT-FAMILY-BLOCKING-CALLEES, adapted from REFCOUNT-REFERENCE-RETURNING-DECLARATIONS, adapted from REFCOUNT-RESOURCE-KIND, adapted from SLOT-MOVE-AND-INTERIOR-BORROW, from W43-ABI-ENUM-AND-CONST-PP, from CALLEE-RETENTION-CLASS1) (from CG10-INDIRECT-ACCESS-SOLE-RESIDUALS):
# `providers/contracts.py sync` then `check` under the shared heavy lock (sync
# runs the resource-view example), output and exits to private/cg22r/sync-NAME.*.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/cg22r"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" python3 providers/contracts.py sync > "$out/sync-$name.log" 2>&1
echo $? > "$out/sync-$name.exit"
python3 providers/contracts.py check > "$out/sync-$name-check.log" 2>&1
echo $? > "$out/sync-$name-check.exit"
echo "sync $(cat "$out/sync-$name.exit") check $(cat "$out/sync-$name-check.exit")"
