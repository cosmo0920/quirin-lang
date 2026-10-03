#!/usr/bin/env bash
# lean.sh NAME -- CG26-TAIL-RESIDUAL-ACTUALS: elaborate QCore.VariadicTailActuals
# and its audit from this worktree's proof/ (dependencies built by lake), output
# to private/cg26r/NAME.log and the exit to private/cg26r/NAME.exit.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$repo/private/cg26r"
cd "$repo/proof" || exit 2
{
  timeout 1800 lake build QCore.VariadicTailActuals && \
  timeout 600 lake env lean AuditVariadicTailActuals.lean
} > "$repo/private/cg26r/$name.log" 2>&1
echo $? > "$repo/private/cg26r/$name.exit"
echo "$name exit $(cat "$repo/private/cg26r/$name.exit")"
