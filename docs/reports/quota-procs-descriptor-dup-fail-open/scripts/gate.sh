#!/usr/bin/env bash
# gate.sh NAME [PART] -- QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN (from
# EXECUTION-QUOTA-LOAD-INDEPENDENCE's scripts/gate.sh): cargo check and the
# standing per-change gate (docs/agents/validation.md), every step's output and
# exit to private/qpdf/gate-NAME/, nothing piped, every step under the shared
# heavy lock. PART is `code` (steps 00-07), `docs` (08-09, after the records)
# or `all` (default). Unlike its source, no arbiter runs beside the suite: the
# card measures the arbiters separately (measure-binary.sh), and validation.md
# asks that a many-job arbiter not share the host with the test suite.
set -u
name="$1"
part="${2:-all}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
private="$repo/private/qpdf"
out="$private/gate-$name"
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
  df -h / > "$out/df-start.txt"
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
  df -h / > "$out/df-end.txt"
  echo done > "$out/code-done"
fi
if [ "$part" != code ]; then
  git rev-parse HEAD > "$out/docs-head"
  step 08-python python3 -m unittest discover -s scripts/tests
  step 09-index python3 scripts/build_docs_index.py --check
  echo done > "$out/docs-done"
fi
