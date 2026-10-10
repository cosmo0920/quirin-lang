#!/usr/bin/env bash
# lean.sh NAME -- NULL-MEMBER-INVARIANT-SCOPE (from NULL-CANDIDATE-TRIAGE) (adapted from NULL-SAFETY-PROPERTY's): under the shared heavy lock, build QCore.NullTriage and run the null audit files with warnings as errors (the form check_lean_audits.py uses).
# Output and exits to private/nmis/lean-NAME.*; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/nmis"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo/proof" || exit 2
flock "$lock" lake build QCore.NullTriage > "$out/lean-$name-build.log" 2>&1
echo $? > "$out/lean-$name-build.exit"
echo "build $(cat "$out/lean-$name-build.exit")"
[ "$(cat "$out/lean-$name-build.exit")" = 0 ] || exit 1
for audit in AuditNullDeref AuditNullTriage; do
  flock "$lock" lake env lean -DwarningAsError=true "$audit.lean" > "$out/lean-$name-$audit.log" 2>&1
  echo $? > "$out/lean-$name-$audit.exit"
  echo "$audit $(cat "$out/lean-$name-$audit.exit")"
done
