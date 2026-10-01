#!/usr/bin/env bash
# gate.sh NAME -- the standing per-change gate (docs/agents/validation.md),
# every step's output and exit to private/cg22/gate-NAME/, nothing piped.
# providers/ is untouched by this card, so the provider gate is not due.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg22/gate-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0
git rev-parse HEAD > "$out/head"
step() {
  local label="$1"; shift
  "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
step submodule git submodule update --init --recursive
step fmt cargo fmt --all -- --check
step clippy cargo clippy --locked --all-targets -- -D warnings
step tests python3 scripts/run_test_binaries.py
step probe cargo run -p qcore-compiler --example reevaluation_probe
step followup cargo run -p qcore-compiler --example reevaluation_followup
step lean python3 scripts/check_lean_audits.py
step python python3 -m unittest discover -s scripts/tests
step docs-index python3 scripts/build_docs_index.py --check
