#!/usr/bin/env bash
# lean.sh NAME TARGET...: build Lean targets in this worktree's proof directory,
# logging to private/cg05ef/NAME.log and the exit to NAME.exit (nothing piped).
name="$1"
shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$repo/private/cg05ef" || exit 2
cd "$repo/proof" || exit 2
timeout 1800 lake build "$@" > "$repo/private/cg05ef/$name.log" 2>&1
echo $? > "$repo/private/cg05ef/$name.exit"
