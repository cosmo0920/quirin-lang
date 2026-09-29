#!/usr/bin/env bash
# W47-FB-DOUBLE-STOP-OBSERVATION: the standing gate (docs/agents/validation.md),
# run after the matrix and index regeneration; every exit captured to its own
# file in the report's gate/ directory, logs private. No provider gate: this
# card touches no provider.
# usage: gate.sh LOG_DIR
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$1"
exits="$here/../gate"
mkdir -p "$logs" "$exits"
cd "$repo" || exit 2
export CARGO_INCREMENTAL=0
export PATH="$HOME/.elan/bin:$PATH"
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$exits/$name.exit"
  echo "$name $(cat "$exits/$name.exit")"
}
step 01-submodule git submodule update --init --recursive
step 02-tests python3 scripts/run_test_binaries.py
step 03-clean cargo clean --profile dev
step 04-fmt cargo fmt --all -- --check
step 05-clippy cargo clippy --all-targets -- -D warnings
step 06-probe cargo run -p qcore-compiler --example reevaluation_probe
step 07-followup cargo run -p qcore-compiler --example reevaluation_followup
step 08-lean python3 scripts/check_lean_audits.py
step 09-clean cargo clean --profile dev
step 10-python python3 -m unittest discover -s scripts/tests
step 11-docs-index python3 scripts/build_docs_index.py --check
/usr/lib/llvm-21/bin/clang --version > "$exits/clang-version.txt" 2>&1
echo $? > "$exits/clang-version.exit"
