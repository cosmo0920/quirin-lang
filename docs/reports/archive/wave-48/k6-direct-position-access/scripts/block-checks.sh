#!/usr/bin/env bash
# block-checks.sh NAME -- K6 (the K6 formula block): the targeted checks the
# integrator named, each in a `build` gate slot with target
# private/k6/target-test; logs and exits under private/k6/NAME-*.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
target="$repo/private/k6/target-test"
cd "$repo" || exit 2
run() {
  local step="$1"
  shift
  CARGO_TARGET_DIR="$target" QCORE_CLANG="${QCORE_CLANG:-/usr/lib/llvm-21/bin/clang}" \
    python3 scripts/gate_slot.py build --receipt "$repo/private/k6/$name-$step.receipt.json" -- \
    "$@" > "$repo/private/k6/$name-$step.log" 2>&1
  echo $? > "$repo/private/k6/$name-$step.exit"
}
run frontend cargo test -p qcore-frontend
run contract cargo test -p qcore-contract --test plugin_guarded_symbols --test formal_binding_descriptions --test cjson_guarded_symbols
run gateway-lib cargo test -p qcore-c-gateway --lib -- address_arrow_place member_handoff elaboration typed_field_access
run gateway-fixtures cargo test -p qcore-c-gateway --test k6_direct_position_access --test access_path_positions --test access_path_position_residuals --test cg06_value_connectives --test cg11_pointer_stores --test return_position_pointer_leaf --test reach_gate_positions
run clippy-frontend cargo clippy -p qcore-frontend --all-targets -- -D warnings
run clippy-gateway cargo clippy -p qcore-c-gateway --all-targets -- -D warnings
