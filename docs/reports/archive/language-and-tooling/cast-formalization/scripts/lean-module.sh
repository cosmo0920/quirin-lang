#!/usr/bin/env bash
# lean-module.sh NAME -- CAST-FORMALIZATION: build QCore.CastSemantics (and its
# one import) and run its audit file, exits to private/castf.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/castf"
cd "$repo/proof" || exit 2
timeout 900 "$HOME/.elan/bin/lake" build QCore.CastSemantics > "$out/lean-$name.log" 2>&1
echo $? > "$out/lean-$name.exit"
timeout 900 "$HOME/.elan/bin/lake" env lean AuditCastSemantics.lean > "$out/lean-audit-$name.log" 2>&1
echo $? > "$out/lean-audit-$name.exit"
echo "build $(cat "$out/lean-$name.exit") audit $(cat "$out/lean-audit-$name.exit")"
