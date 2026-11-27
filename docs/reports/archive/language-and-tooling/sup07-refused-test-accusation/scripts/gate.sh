#!/usr/bin/env bash
# gate.sh NAME -- SUP07-REFUSED-TEST-ACCUSATION (from MONKEY-VHOST-PROPERTY-CUSTODY):
# cargo check and the standing per-change gate (docs/agents/validation.md),
# every step's output and exit to private/sup07/gate-NAME/, nothing piped.
# providers/ is unchanged, so the provider gate is not due. Each step runs
# under the shared heavy lock.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/sup07/gate-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
git rev-parse HEAD > "$out/head"
step() {
  local label="$1"; shift
  flock "$lock" "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
step 00-check cargo check --workspace --all-targets
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
flock "$lock" cargo clean --profile dev > "$out/clean.log" 2>&1
echo done > "$out/done"
