#!/usr/bin/env bash
# The standing gate (docs/agents/validation.md), every exit captured to its own
# file, logs private. No provider step: this card does not touch providers/.
# usage: gate.sh LOG_DIR
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$1"
mkdir -p "$logs"
cd "$repo" || exit 2
export CARGO_INCREMENTAL=0
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
step submodule git submodule update --init --recursive
step tests python3 scripts/run_test_binaries.py
step clean-dev cargo clean --profile dev
step fmt cargo fmt --all -- --check
step clippy cargo clippy --all-targets -- -D warnings
step reevaluation-probe cargo run -p qcore-compiler --example reevaluation_probe
step reevaluation-followup cargo run -p qcore-compiler --example reevaluation_followup
step lean python3 scripts/check_lean_audits.py
step python python3 -m unittest discover -s scripts/tests
step docs-index python3 scripts/build_docs_index.py --check
step clang-version /usr/lib/llvm-21/bin/clang --version
