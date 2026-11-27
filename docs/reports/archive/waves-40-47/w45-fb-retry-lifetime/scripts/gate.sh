#!/usr/bin/env bash
# The standing per-change gate (docs/agents/validation.md), each step's exit to
# its own file, never piped. usage: gate.sh LOG_DIR (fresh)
set -u
dir="$1"
mkdir "$dir" || exit 2
export CARGO_INCREMENTAL=0
step() {
  local name="$1"; shift
  "$@" > "$dir/$name.log" 2>&1
  echo $? > "$dir/$name.exit"
  echo "$name $(cat "$dir/$name.exit")"
}
step submodule git submodule update --init --recursive
step tests python3 scripts/run_test_binaries.py
step clean cargo clean --profile dev
step fmt cargo fmt --all -- --check
step clippy cargo clippy --all-targets -- -D warnings
step probe cargo run -p qcore-compiler --example reevaluation_probe
step followup cargo run -p qcore-compiler --example reevaluation_followup
step lean python3 scripts/check_lean_audits.py
step python python3 -m unittest discover -s scripts/tests
step docs_index python3 scripts/build_docs_index.py --check
