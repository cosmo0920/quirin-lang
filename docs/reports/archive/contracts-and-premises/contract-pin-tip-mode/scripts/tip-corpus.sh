#!/bin/bash
# usage: tip-corpus.sh LABEL  -- corpus arbiter into private/contract-pin-tip-mode/LABEL
set -u
W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-abb94a55c5f4a07cd
L=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
P=$W/private/contract-pin-tip-mode
OUT=$P/$1
mkdir -p "$P"
rm -rf "$OUT"
cd "$W"
QCORE_TYPED_PATH_PARITY=$OUT/parity.jsonl flock "$L" python3 scripts/run_pinned_audit.py "$OUT" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 4 > "$P/$1.log" 2>&1
echo $? > "$P/$1.exit"
