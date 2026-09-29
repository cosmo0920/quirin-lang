#!/bin/sh
# CG10-COMMON-TYPED-PLACE: the standing gate (docs/agents/validation.md).
# usage: gate.sh LOG_DIR   (relative to the checkout root). Every exit is
# captured to LOG_DIR/<step>.exit; no step is piped. providers/ is unchanged,
# so the provider gate is not due.
set -u
root="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$root" || exit 2
logs="$root/$1"
mkdir -p "$logs"
step() {
  name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
}
export CARGO_INCREMENTAL=0
step 01-submodule git submodule update --init --recursive
step 02-fmt cargo fmt --all -- --check
step 03-tests python3 scripts/run_test_binaries.py
step 05-clippy cargo clippy --all-targets -- -D warnings
step 06-probe cargo run -p qcore-compiler --example reevaluation_probe
step 07-followup cargo run -p qcore-compiler --example reevaluation_followup
step 08-lean python3 scripts/check_lean_audits.py
step 10-python python3 -m unittest discover -s scripts/tests
step 11-docs-index python3 scripts/build_docs_index.py --check
step 12-clean cargo clean --profile dev
clang --version > "$logs/clang-version.txt" 2>&1
echo $? > "$logs/clang-version.exit"
