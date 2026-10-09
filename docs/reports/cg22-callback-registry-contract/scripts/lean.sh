#!/usr/bin/env bash
# lean.sh NAME -- REFCOUNT-REFERENCE-RETURNING-DECLARATIONS: under the shared
# heavy lock, build QCore.ReferenceResult and run its audit file with warnings
# as errors (the form check_lean_audits.py uses). Output and exits to
# private/cg22r/lean-NAME.*; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg22r"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo/proof" || exit 2
flock "$lock" lake build QCore.ReferenceResult > "$out/lean-$name-build.log" 2>&1
echo $? > "$out/lean-$name-build.exit"
echo "build $(cat "$out/lean-$name-build.exit")"
[ "$(cat "$out/lean-$name-build.exit")" = 0 ] || exit 1
flock "$lock" lake env lean -DwarningAsError=true AuditReferenceResult.lean > "$out/lean-$name-audit.log" 2>&1
echo $? > "$out/lean-$name-audit.exit"
echo "audit $(cat "$out/lean-$name-audit.exit")"
