#!/usr/bin/env bash
# build.sh NAME [TEST-FILTER] -- CAST-FORMALIZATION: under the shared heavy lock,
# `cargo check` the gateway with its tests, run the gateway's unit tests
# matching TEST-FILTER (default cast_semantics), then build the release
# quirin-audit. Every exit is written to a file under private/castf; nothing is
# piped.
set -u
name="$1"
filter="${2:-cast_semantics}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/castf"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" bash -c '
  cargo test -p qcore-c-gateway --lib "$2" > "$1/unit-$3.log" 2>&1; echo $? > "$1/unit-$3.exit"
  [ "$(cat "$1/unit-$3.exit")" = 0 ] || exit 1
  cargo build --release -p qcore-audit --bin quirin-audit > "$1/build-$3.log" 2>&1; echo $? > "$1/build-$3.exit"
' _ "$out" "$filter" "$name"
echo "unit $(cat "$out/unit-$name.exit" 2>/dev/null) build $(cat "$out/build-$name.exit" 2>/dev/null)"
