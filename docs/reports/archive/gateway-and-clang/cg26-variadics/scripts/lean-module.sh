#!/usr/bin/env bash
# lean-module.sh NAME -- build QCore.Variadics (and its imports) with the pinned
# toolchain; log and exit under private/cg26va/. The full audit set is the
# standing gate's `python3 scripts/check_lean_audits.py`.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg26va"
mkdir -p "$out" || exit 2
cd "$repo/proof" || exit 2
timeout 1800 lake build QCore.Variadics > "$out/lean-$name.log" 2>&1
echo $? > "$out/lean-$name.exit"
cat "$out/lean-$name.exit"
