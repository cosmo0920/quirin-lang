#!/usr/bin/env bash
# W45-SR5-SOUNDNESS standing gate (adapted from STABILITY-REEVAL-5) (docs/agents/validation.md). Every exit is
# captured to its own file, no gate step is piped, logs are worktree-private.
# This card touches nothing under providers/, so the provider gate is not run.
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
  echo "$name $(cat "$logs/$name.exit")" >> "$logs/steps.txt"
}
git rev-parse HEAD > "$logs/head.txt"
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
step clang-version clang --version
echo done > "$logs/ALL_DONE"
