#!/usr/bin/env bash
# gate.sh NAME [PART] -- RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION (adapted from
# OP-APP-RK-CALLBACKS-VIA-POLL-CB's gate.sh): cargo check and the standing
# per-change gate (docs/agents/validation.md), every step's output and exit to
# private/rkou/gate-NAME/, nothing piped. PART is `code` (00-07), `docs`
# (08-09, after the records and the matrix regeneration) or `all`. No
# provider gate: providers/ is unchanged. Run under one exclusive flock on the
# shared heavy lock with HELD=1 (gate-run.sh), so no step takes it again.
set -u
name="$1"
part="${2:-all}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
private="$repo/private/rkou"
out="$private/gate-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  if [ "${HELD:-0}" = 1 ]; then
    "$@" > "$out/$label.log" 2>&1
  else
    flock "$lock" "$@" > "$out/$label.log" 2>&1
  fi
  echo $? > "$out/$label.exit"
}
if [ "$part" != docs ]; then
  git rev-parse HEAD > "$out/head"
  git status --porcelain --untracked-files=no > "$out/dirty"
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
  echo done > "$out/code-done"
fi
if [ "$part" != code ]; then
  git rev-parse HEAD > "$out/docs-head"
  step 08-python python3 -m unittest discover -s scripts/tests
  step 09-index python3 scripts/build_docs_index.py --check
  echo done > "$out/docs-done"
fi
