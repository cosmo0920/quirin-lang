#!/usr/bin/env bash
# lean.sh [MODULE] -- MEMBER-PREPASS-OTHER-ROUTES: under the shared heavy
# lock, build one Lean module of proof/ (default QCore.MemberPrepassRoutes)
# and its audit file, output to private/mpor/lean-*.log, exits to files.
set -u
module="${1:-QCore.MemberPrepassRoutes}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/mpor"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo/proof" || exit 2
flock "$lock" lake build "$module" > "$out/lean-module.log" 2>&1
echo $? > "$out/lean-module.exit"
audit="Audit${module#QCore.}.lean"
if [ -f "$audit" ]; then
  flock "$lock" lake env lean "$audit" > "$out/lean-audit.log" 2>&1
  echo $? > "$out/lean-audit.exit"
fi
cat "$out/lean-module.exit"
