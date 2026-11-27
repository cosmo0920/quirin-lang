#!/usr/bin/env bash
# gate.sh NAME -- LEAN-4-35-TOOLCHAIN (from OP-CALLBACK-MEMBER-PATH-RELEASE's
# scripts/gate.sh, provider steps dropped because providers/ is unchanged):
# cargo check and the standing per-change gate (docs/agents/validation.md),
# every step's output and exit to private/lean-4-35-toolchain/gate-NAME/,
# nothing piped. Run under one hold of the shared heavy lock (gate-whole.sh).
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
private="$repo/private/lean-4-35-toolchain"
out="$private/gate-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
git rev-parse HEAD > "$out/head"
git status --short > "$out/tree-status.txt"
cat proof/lean-toolchain > "$out/lean-toolchain.txt"
(cd proof && lake env lean --version) > "$out/lean-version.txt" 2>&1
step 00-check cargo check --workspace --all-targets
step 01-submodule git submodule update --init --recursive
step 02-fmt cargo fmt --all -- --check
step 03-clippy cargo clippy --locked --all-targets -- -D warnings
step 04-tests python3 scripts/run_test_binaries.py --log-dir "$private/tests-$name"
step 05-probe cargo run -p qcore-compiler --example reevaluation_probe
step 06-followup cargo run -p qcore-compiler --example reevaluation_followup
step 07-lean python3 scripts/check_lean_audits.py
clang --version > "$out/clang-version.txt" 2>&1
echo $? > "$out/clang-version.exit"
step 08-python python3 -m unittest discover -s scripts/tests
step 09-index python3 scripts/build_docs_index.py --check
echo done > "$out/done"
