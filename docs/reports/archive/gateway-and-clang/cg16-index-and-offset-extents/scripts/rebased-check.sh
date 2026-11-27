#!/bin/sh
# CG16-INDEX-AND-OFFSET-EXTENTS: the targeted check after the last rebase
# (main moved with disjoint gated commits after the full gate): the workspace
# check, the card's own test files, the Lean audits, under the shared lock;
# then the dev profile cleaned. usage: rebased-check.sh LOCK LOG_DIR. Every
# exit is captured to a file; nothing is piped.
set -u
lock="$1"
root="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$root" || exit 2
logs="$root/$2"
mkdir -p "$logs"
CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 flock "$lock" sh -c '
  cargo check --workspace --all-targets > "$1/r1-check.log" 2>&1
  echo $? > "$1/r1-check.exit"
  cargo test -p qcore-core --test index_extent > "$1/r2-tests-core.log" 2>&1
  echo $? > "$1/r2-tests-core.exit"
  cargo test -p qcore-contract --test bounds_contract > "$1/r2-tests-contract.log" 2>&1
  echo $? > "$1/r2-tests-contract.exit"
  cargo test -p qcore-c-gateway --test cg16_index_extents --test clang_gap_phase3 > "$1/r3-tests-gateway.log" 2>&1
  echo $? > "$1/r3-tests-gateway.exit"
  python3 scripts/check_lean_audits.py > "$1/r4-lean.log" 2>&1
  echo $? > "$1/r4-lean.exit"
' check "$logs"
cargo clean --profile dev > "$logs/r5-clean.log" 2>&1
echo $? > "$logs/r5-clean.exit"
