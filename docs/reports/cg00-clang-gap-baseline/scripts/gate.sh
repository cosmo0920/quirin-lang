#!/bin/sh
# CG00-CLANG-GAP-BASELINE: the standing gate steps after the test runner.
# usage: gate.sh LOG_DIR   (relative to the checkout root). Every exit is
# captured to LOG_DIR/<step>.exit; no step is piped.
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
step 01-submodule git submodule update --init --recursive
step 02-fmt cargo fmt --all -- --check
step 05-clippy cargo clippy --all-targets -- -D warnings
step 06-probe cargo run -p qcore-compiler --example reevaluation_probe
step 07-followup cargo run -p qcore-compiler --example reevaluation_followup
step 08-lean python3 scripts/check_lean_audits.py
clang --version > "$logs/clang-version.txt" 2>&1
echo $? > "$logs/clang-version.exit"
