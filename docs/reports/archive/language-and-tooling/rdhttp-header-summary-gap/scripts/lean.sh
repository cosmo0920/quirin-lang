#!/usr/bin/env bash
# lean.sh NAME -- RDHTTP-HEADER-SUMMARY-GAP: build QCore.DeepUnitAst and run its
# audit under the shared heavy lock into private/rhsg/lean-NAME.{log,exit}.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo/proof" || exit 2
flock "$lock" lake build QCore.DeepUnitAst > "$repo/private/rhsg/lean-$name.log" 2>&1
echo $? > "$repo/private/rhsg/lean-$name.exit"
cat "$repo/private/rhsg/lean-$name.exit"
if [ -f AuditDeepUnitAst.lean ]; then
  flock "$lock" lake env lean AuditDeepUnitAst.lean > "$repo/private/rhsg/lean-audit-$name.log" 2>&1
  echo $? > "$repo/private/rhsg/lean-audit-$name.exit"
  cat "$repo/private/rhsg/lean-audit-$name.exit"
fi
