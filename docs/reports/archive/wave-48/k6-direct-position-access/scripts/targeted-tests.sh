#!/usr/bin/env bash
# targeted-tests.sh NAME -- K6: the card's targeted checks, each in a `build`
# gate slot with target private/k6/target-test; logs and exits under private/k6/.
#   NAME-fixtures: the new fixture file and every existing test file of the
#     rules rule 81 amends or whose readers it changes (10k, 10n, 10z, 5b's
#     may_bind consumers, the elaborator, rule 75), and every file spelling
#     `(&p->q)->f` (cg11_pointer_stores, access_path_position_residuals);
#   NAME-lib: the library unit tests of the touched modules.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
target="$repo/private/k6/target-test"
cd "$repo" || exit 2
tests=(
  k6_direct_position_access
  access_path_positions
  access_path_position_residuals
  cg06_value_connectives
  cg06_connective_footprints
  cg02_elaboration
  cg05_elaborator_call_footprints
  cg11_pointer_stores
  reach_gate_positions
  return_position_pointer_leaf
  return_position_modeled_call
  neutral_expression_positions
  intrusive_list_iteration
  k1_call_actual_access
  k5_local_origin_aliases
  k7_residual_causes
  parenthesised_callee
)
args=()
for t in "${tests[@]}"; do args+=(--test "$t"); done
CARGO_TARGET_DIR="$target" QCORE_CLANG="${QCORE_CLANG:-/usr/lib/llvm-21/bin/clang}" \
  python3 scripts/gate_slot.py build --receipt "$repo/private/k6/$name-fixtures.receipt.json" -- \
  cargo test -p qcore-c-gateway "${args[@]}" > "$repo/private/k6/$name-fixtures.log" 2>&1
echo $? > "$repo/private/k6/$name-fixtures.exit"
CARGO_TARGET_DIR="$target" QCORE_CLANG="${QCORE_CLANG:-/usr/lib/llvm-21/bin/clang}" \
  python3 scripts/gate_slot.py build --receipt "$repo/private/k6/$name-lib.receipt.json" -- \
  cargo test -p qcore-c-gateway --lib -- address_arrow_place elaboration typed_field_access \
  typed_position_shapes typed_root_declarations readonly_actual_frame value_connective \
  > "$repo/private/k6/$name-lib.log" 2>&1
echo $? > "$repo/private/k6/$name-lib.exit"
