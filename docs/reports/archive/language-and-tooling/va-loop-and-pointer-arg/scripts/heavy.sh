#!/usr/bin/env bash
# heavy.sh NAME -- the card's gate in one acquisition of the shared lock (run as
# `flock LOCK bash heavy.sh NAME`): `cargo check --workspace --all-targets`,
# the standing gate (gate.sh into private/valoop/NAME), then
# `cargo clean --profile dev`. Every exit is captured to a file; nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/valoop"
mkdir -p "$out/$name" || exit 2
cd "$repo" || exit 2
df -h / > "$out/$name-df-start.txt" 2>&1
CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 cargo check --workspace --all-targets > "$out/$name/00-check.log" 2>&1
echo $? > "$out/$name/00-check.exit"
CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 sh "$here/gate.sh" "private/valoop/$name"
df -h / > "$out/$name-df-end.txt" 2>&1
cargo clean --profile dev > "$out/$name-clean.log" 2>&1
echo $? > "$out/$name-clean.exit"
echo done > "$out/$name.done"
