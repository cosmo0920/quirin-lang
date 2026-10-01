#!/usr/bin/env bash
# heavy.sh NAME -- the card's heavy steps in one acquisition of the shared lock
# (run as `flock LOCK bash heavy.sh NAME`): the rebased tree's
# `cargo check --workspace --all-targets`, then the standing gate
# (scripts/gate.sh into private/cg26va/NAME), then `cargo clean --profile dev`.
# Every exit is captured to a file; nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg26va"
cd "$repo" || exit 2
df -h / > "$out/$name-df-start.txt" 2>&1
bash "$here/check.sh" "$name"
sh "$here/gate.sh" "private/cg26va/$name"
df -h / > "$out/$name-df-end.txt" 2>&1
cargo clean --profile dev > "$out/$name-clean.log" 2>&1
echo $? > "$out/$name-clean.exit"
echo done > "$out/$name.done"
