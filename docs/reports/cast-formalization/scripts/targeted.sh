#!/usr/bin/env bash
# targeted.sh NAME -- CAST-FORMALIZATION: under the shared heavy lock, the
# gateway's unit tests of the card's modules and its neighbours' integration
# tests, then `cargo clean --profile dev`. Exits to private/castf.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/castf"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" bash -c '
  cargo test --no-fail-fast -p qcore-c-gateway --lib -- cast_semantics legacy_return_family void_view cast_representation elaboration > "$1/targeted-unit-$2.log" 2>&1
  echo $? > "$1/targeted-unit-$2.exit"
  cargo test --no-fail-fast -p qcore-c-gateway --test cast_formalization --test cg12_void_view_readback --test cg08_casts_and_type_queries --test cg26_variadics --test index_traversal --test sub_object_stores --test return_expression_families -- --nocapture > "$1/targeted-int-$2.log" 2>&1
  echo $? > "$1/targeted-int-$2.exit"
' _ "$out" "$name"
echo "unit $(cat "$out/targeted-unit-$name.exit") integration $(cat "$out/targeted-int-$name.exit")"
