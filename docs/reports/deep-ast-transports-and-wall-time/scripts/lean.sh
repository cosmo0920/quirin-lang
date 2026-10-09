#!/usr/bin/env bash
# lean.sh NAME -- DEEP-AST-TRANSPORTS-AND-WALL-TIME (from RDHTTP-HEADER-SUMMARY-GAP's
# scripts/lean.sh): build QCore.DeepAstTransports and run its audit under the
# shared heavy lock into private/dat/lean-NAME.{log,exit}.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo/proof" || exit 2
flock "$lock" lake build QCore.DeepAstTransports > "$repo/private/dat/lean-$name.log" 2>&1
echo $? > "$repo/private/dat/lean-$name.exit"
cat "$repo/private/dat/lean-$name.exit"
flock "$lock" lake env lean AuditDeepAstTransports.lean > "$repo/private/dat/lean-audit-$name.log" 2>&1
echo $? > "$repo/private/dat/lean-audit-$name.exit"
cat "$repo/private/dat/lean-audit-$name.exit"
