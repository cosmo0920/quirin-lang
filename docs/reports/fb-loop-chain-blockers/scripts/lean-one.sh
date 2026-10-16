#!/usr/bin/env bash
# lean-one.sh FILE -- FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS: elaborate one
# Lean file from the pinned proof directory (an iteration check while writing
# the module; the gate's check_lean_audits.py is the arbiter). Output and exit
# to private/fblb/lean-one.{log,exit}.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/fblb"
mkdir -p "$out" || exit 2
cd "$repo/proof" || exit 2
lake env lean "$1" > "$out/lean-one.log" 2>&1
echo $? > "$out/lean-one.exit"
cat "$out/lean-one.exit"
