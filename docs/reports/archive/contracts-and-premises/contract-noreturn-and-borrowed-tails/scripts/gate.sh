#!/usr/bin/env bash
# gate.sh NAME [PART] -- CONTRACT-NORETURN-AND-BORROWED-TAILS (from
# W48-BOTTOM-UP-CALLEE-BODIES): cargo check and the standing per-change gate
# (docs/agents/validation.md), every step's output and exit to
# private/cnbt/gate-NAME/, nothing piped. PART is `code` (steps 00-07, the Rust
# and Lean steps), `docs` (08-09, run after the records) or `all` (default).
# The provider gate is scripts/provider-gate.sh. Each step runs under the
# shared heavy lock.
set -u
name="$1"
part="${2:-all}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/cnbt/gate-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  flock "$lock" "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
if [ "$part" != docs ]; then
  git rev-parse HEAD > "$out/head"
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
  flock "$lock" cargo clean --profile dev > "$out/clean.log" 2>&1
  echo done > "$out/code-done"
fi
if [ "$part" != code ]; then
  git rev-parse HEAD > "$out/docs-head"
  step 08-python python3 -m unittest discover -s scripts/tests
  step 09-index python3 scripts/build_docs_index.py --check
  echo done > "$out/docs-done"
fi
