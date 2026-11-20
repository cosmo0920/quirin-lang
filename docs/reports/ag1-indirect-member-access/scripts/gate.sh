#!/usr/bin/env bash
# gate.sh N -- AG1: the scoped card gate against the card base, logs and exit
# under private/gate-N (the standing gate, docs/agents/validation.md).
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
python3 scripts/gate_scope.py --base a9c6dcf44 --out "$repo/private/gate-$1" \
  > "$repo/private/gate-$1.console.log" 2>&1
echo $? > "$repo/private/gate-$1.exit"
