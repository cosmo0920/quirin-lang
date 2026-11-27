#!/usr/bin/env bash
# census.sh NAME -- ROUTE-SELECTION-ORDER-RESTATED (from QR-LEXICAL-SELECTORS-DECLARATION-NAMES): the route census of
# every tracked Quirin source (compiler/tests/qr_print_route_census.rs) on the
# checked-out tree, under the shared heavy lock, into
# private/rsor/census-NAME.jsonl; the head, the dirty state, the log and the
# exit beside it. Nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/rsor"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
git rev-parse HEAD > "$out/census-$name.head"
git status --porcelain --untracked-files=no > "$out/census-$name.dirty"
QR_ROUTE_CENSUS_OUT="$out/census-$name.jsonl" flock "$lock" \
  cargo test -p qcore-compiler --test qr_print_route_census -- --ignored \
  > "$out/census-$name.log" 2>&1
echo $? > "$out/census-$name.exit"
cat "$out/census-$name.exit"
