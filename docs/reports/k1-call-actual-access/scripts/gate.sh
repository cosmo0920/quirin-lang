#!/usr/bin/env bash
# gate.sh N -- K1: the scoped card gate on this worktree's tip against the card
# base, run directory private/gate-N; console and exit under private/k1/.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
n="$1"
cd "$repo" || exit 2
python3 scripts/gate_scope.py --base 3ca85c7a6 --out "$repo/private/gate-$n" \
  > "$repo/private/k1/gate-$n.console.log" 2>&1
echo $? > "$repo/private/k1/gate-$n.exit"
