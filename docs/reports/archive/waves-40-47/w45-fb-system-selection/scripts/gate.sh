#!/usr/bin/env bash
# W45-FB-SYSTEM-SELECTION standing gate (docs/agents/validation.md). Every
# step's exit goes to its own file; nothing is piped.
# usage: gate.sh LOG_DIR
set -u
log="$1"
mkdir -p "$log"
export CARGO_INCREMENTAL=0
step() {
  local name="$1"; shift
  "$@" > "$log/$name.log" 2>&1
  echo $? > "$log/$name.exit"
}
step submodule git submodule update --init --recursive
step rust-suite python3 scripts/run_test_binaries.py
step clean-dev cargo clean --profile dev
step fmt cargo fmt --all -- --check
step clippy cargo clippy --all-targets -- -D warnings
step probe cargo run -p qcore-compiler --example reevaluation_probe
step followup cargo run -p qcore-compiler --example reevaluation_followup
step lean python3 scripts/check_lean_audits.py
step python python3 -m unittest discover -s scripts/tests
step docs-index python3 scripts/build_docs_index.py --check
step clang-version clang --version
