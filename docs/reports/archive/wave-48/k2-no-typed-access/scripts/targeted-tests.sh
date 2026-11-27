#!/usr/bin/env bash
# targeted-tests.sh NAME -- K2: the card's targeted checks (no card gate, user
# 2026-11-24): the gateway unit tests, the rule-79 fixtures, every existing
# gateway test file holding a `( NAME * sizeof (` run or a `sizeof(T) <op>`
# comparison (grep of crates/qcore-c-gateway/tests), the rendered-table block
# tests of qcore-contract and the qcore-frontend suite; each in a `build` gate
# slot, target under private/k2/target-test, logs and exits under private/k2.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
out="$repo/private/k2/$name"
mkdir -p "$out"
cd "$repo" || exit 2
run() {
  local label="$1"; shift
  CARGO_TARGET_DIR="$repo/private/k2/target-test" python3 scripts/gate_slot.py build \
    --receipt "$out/$label.receipt.json" -- "$@" > "$out/$label.log" 2>&1
  local code=$?
  echo "$label $code $(grep -E '^test result' "$out/$label.log" | awk '{p+=$4; f+=$6} END {print p" passed "f" failed"}')" | tee -a "$out/summary.txt"
}
: > "$out/summary.txt"
run gateway-lib cargo test -p qcore-c-gateway --lib
for t in k2_no_typed_access cg08_casts_and_type_queries neutral_expression_positions scalar_statement_forms sizeof_operand; do
  run "gateway-$t" cargo test -p qcore-c-gateway --test "$t"
done
for t in plugin_guarded_symbols formal_binding_descriptions cjson_guarded_symbols; do
  run "contract-$t" cargo test -p qcore-contract --test "$t"
done
run frontend cargo test -p qcore-frontend
