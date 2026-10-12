#!/usr/bin/env bash
# selector-census.sh NAME -- ROUTE-SELECTION-ORDER-RESTATED (from QR-LEXICAL-SELECTORS-DECLARATION-NAMES): the
# declaration-name census of every route selector
# (compiler/tests/qr_lexical_selector_census.rs) on the checked-out tree,
# under the shared heavy lock, into private/rsor/selector-census-NAME.jsonl;
# the head, the dirty state, the log and the exit beside it. Nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/rsor"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
git rev-parse HEAD > "$out/selector-census-$name.head"
git status --porcelain --untracked-files=no > "$out/selector-census-$name.dirty"
QR_SELECTOR_CENSUS_OUT="$out/selector-census-$name.jsonl" flock "$lock" \
  cargo test -p qcore-compiler --test qr_lexical_selector_census -- --ignored \
  > "$out/selector-census-$name.log" 2>&1
echo $? > "$out/selector-census-$name.exit"
cat "$out/selector-census-$name.exit"
