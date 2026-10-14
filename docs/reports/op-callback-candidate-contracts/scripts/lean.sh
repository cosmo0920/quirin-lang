#!/usr/bin/env bash
# lean.sh [TARGET...] -- OP-CONSTRUCTOR-DECLARATIONS: build Lean targets of
# this worktree's proof project (default QCore.OpCallbacks and the audit
# module), from inside proof/ so elan reads proof/lean-toolchain. The log goes
# to private/opcand/logs/lean.log and the exit to lean.exit.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$repo/private/opcand/logs"
mkdir -p "$logs" || exit 2
cd "$repo/proof" || exit 2
if [ "$#" -eq 0 ]; then
  set -- QCore.OpCallbacks QCore.BodyClauseStanding
fi
lake build "$@" > "$logs/lean.log" 2>&1
echo $? > "$logs/lean.exit"
cat "$logs/lean.exit"
