#!/usr/bin/env bash
# gate.sh NAME -- the standing per-change gate (docs/agents/validation.md),
# every step's output and exit to private/cg22/gate-NAME/ (exits and the Clang
# recording are copied into the report's gate/ directory), nothing piped.
# providers/ is untouched by this card, so the provider gate is not due.
# Run under the shared heavy lock (the coordinator's instruction, 2026-10-02).
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg22/gate-$name"
record="$here/../gate"
mkdir -p "$out" "$record" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0
git rev-parse HEAD > "$out/head"
step() {
  local label="$1"; shift
  "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
step 01-submodule git submodule update --init --recursive
step 02-fmt cargo fmt --all -- --check
step 03-clippy cargo clippy --locked --all-targets -- -D warnings
step 04-tests python3 scripts/run_test_binaries.py
step 05-probe cargo run -p qcore-compiler --example reevaluation_probe
step 06-followup cargo run -p qcore-compiler --example reevaluation_followup
step 07-lean python3 scripts/check_lean_audits.py
clang --version > "$out/clang-version.txt" 2>&1
echo $? > "$out/clang-version.exit"
step 08-python python3 -m unittest discover -s scripts/tests
step 09-index python3 scripts/build_docs_index.py --check
cargo clean --profile dev > "$out/clean.log" 2>&1
cp "$out"/*.exit "$out/head" "$out/clang-version.txt" "$record/"
