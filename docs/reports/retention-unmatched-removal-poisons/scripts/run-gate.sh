#!/usr/bin/env bash
# run-gate.sh [NAME] -- RETENTION-UNMATCHED-REMOVAL-POISONS: take the shared
# heavy lock exclusively and run gate-whole.sh NAME (default g1) inside it.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
flock "$lock" bash "$here/gate-whole.sh" "${1:-g1}"
