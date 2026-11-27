#!/usr/bin/env bash
# gate.sh N -- K7: the scoped card gate against the card base, into
# private/gate-N, console and exit beside it under private/k7/.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
n="$1"
mkdir -p "$repo/private/k7"
cd "$repo" || exit 2
python3 scripts/gate_scope.py --base 3ca85c7a6 --out "$repo/private/gate-$n" \
  > "$repo/private/k7/gate-$n.console.log" 2>&1
echo $? > "$repo/private/k7/gate-$n.exit"
