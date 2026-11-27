#!/usr/bin/env bash
# targeted.sh NAME -- CG10-INDIRECT-ACCESS-SOLE-BLOCKERS: under the shared heavy
# lock, the card's unit tests (record_object_place) and integration tests
# (indirect_access_sole_blockers), then the neighbouring gateway suites whose
# routes the card touches, each to its own log and exit under private/cg10sole.
# Nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg10sole/targeted-$name"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  flock "$lock" "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
step unit cargo test -p qcore-c-gateway --lib record_object_place
step card cargo test -p qcore-c-gateway --test indirect_access_sole_blockers
for suite in access_path_statements access_path_positions bound_untracked_roots \
  call_argument_access return_position_modeled_call cg11_pointer_stores cg14_aggregate_values \
  union_bitfield_places typed_place; do
  if [ -f "crates/qcore-c-gateway/tests/$suite.rs" ]; then
    step "suite-$suite" cargo test -p qcore-c-gateway --test "$suite"
  fi
done
for f in "$out"/*.exit; do echo "$(basename "$f" .exit) $(cat "$f")"; done
