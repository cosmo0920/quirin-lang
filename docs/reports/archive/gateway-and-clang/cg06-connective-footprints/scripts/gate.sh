#!/bin/sh
# CG06-VALUE-CONNECTIVE-RESIDUALS (sibling-effect part): the standing gate (docs/agents/validation.md).
# usage: gate.sh LOG_DIR   (relative to the checkout root). Every exit is
# captured to LOG_DIR/<step>.exit; no step is piped. The matrix regeneration
# and the Python suite run afterwards, from the gate.json these figures fill
# (the Python suite reads the committed matrix).
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
export CARGO_BUILD_JOBS=6
export CARGO_INCREMENTAL=0
step 01-submodule git submodule update --init --recursive
step 02-tests python3 scripts/run_test_binaries.py
step 03-fmt cargo fmt --all -- --check
step 04-clippy cargo clippy --all-targets -- -D warnings
step 05-probe cargo run -p qcore-compiler --example reevaluation_probe
step 06-followup cargo run -p qcore-compiler --example reevaluation_followup
step 07-lean python3 scripts/check_lean_audits.py
clang --version > "$logs/clang-version.txt" 2>&1
echo $? > "$logs/clang-version.exit"
