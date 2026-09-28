#!/usr/bin/env bash
# W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT standing gate (docs/agents/validation.md;
# adapted from w47-callee-closure-r2/scripts/gate.sh). No providers/ path is
# touched, so the provider gate does not apply. Every exit is captured to its
# own file, no gate step is piped, logs are private.
# usage: gate.sh LOG_DIR [code|docs]
#   code: submodule, tests, clean, fmt, clippy, census-feature clippy and
#         library tests, both reevaluation examples, Lean audits, clang;
#   docs: the Python suite and the docs-index check (after the matrix and
#         index regeneration).
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$1"
part="${2:-code}"
mkdir -p "$logs"
cd "$repo" || exit 2
export CARGO_INCREMENTAL=0
export PATH="$HOME/.elan/bin:$PATH"
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")" >> "$logs/steps.txt"
}
git rev-parse HEAD > "$logs/head-$part.txt"
if [ "$part" = code ]; then
  step submodule git submodule update --init --recursive
  step tests python3 scripts/run_test_binaries.py
  step clean-dev cargo clean --profile dev
  step fmt cargo fmt --all -- --check
  step clippy cargo clippy --all-targets -- -D warnings
  step census-clippy cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -- -D warnings
  step census-lib-tests cargo test -p qcore-c-gateway --features indirect-census --lib
  step reevaluation-probe cargo run -p qcore-compiler --example reevaluation_probe
  step reevaluation-followup cargo run -p qcore-compiler --example reevaluation_followup
  step lean python3 -B scripts/check_lean_audits.py
  step clang-version clang --version
else
  step python python3 -m unittest discover -s scripts/tests
  step docs-index python3 scripts/build_docs_index.py --check
fi
echo done > "$logs/ALL_DONE-$part"
