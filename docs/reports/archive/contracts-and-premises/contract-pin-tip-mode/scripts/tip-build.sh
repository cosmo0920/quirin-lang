#!/bin/bash
# usage: tip-build.sh LABEL  -- release build of quirin-audit under the disk lock
set -u
W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-abb94a55c5f4a07cd
L=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
P=$W/private/contract-pin-tip-mode
mkdir -p "$P"
cd "$W"
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$L" cargo build --release -p qcore-audit --bin quirin-audit > "$P/$1-build.log" 2>&1
echo $? > "$P/$1-build.exit"
