#!/usr/bin/env bash
# gate.sh NAME [PART] -- EXECUTION-QUOTA-LOAD-INDEPENDENCE (from
# GAPO-RETURN-CORRECTNESS's scripts/gate.sh): cargo check and the standing
# per-change gate (docs/agents/validation.md), every step's output and exit to
# private/eqli/gate-NAME/, nothing piped. PART is `code` (steps 00-07) or
# `docs` (08-09, after the records) or `all` (default).
#
# The card's difference: step 04 (the whole Rust suite through
# scripts/run_test_binaries.py) runs CONCURRENTLY with the AFTER corpus
# arbiter (measure-binary.sh NAME-corpus BINARY corpus, --jobs 4), which the
# card requires as its proof under load. The arbiter takes the shared heavy
# lock; step 04 deliberately does not, so the two overlap. Epoch timestamps of
# both are written to overlap.txt. Every other step runs under the lock.
# BINARY (required for `code`) is the release quirin-audit built from this tip.
set -u
name="$1"
part="${2:-all}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
private="$repo/private/eqli"
out="$private/gate-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  flock "$lock" "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
stamp() { echo "$1 $(date +%s.%N)" >> "$out/overlap.txt"; }
if [ "$part" != docs ]; then
  : "${BINARY:?BINARY must name the release quirin-audit built from this tip}"
  git rev-parse HEAD > "$out/head"
  step 00-check cargo check --workspace --all-targets
  step 01-submodule git submodule update --init --recursive
  step 02-fmt cargo fmt --all -- --check
  step 03-clippy cargo clippy --locked --all-targets -- -D warnings
  # Build the suite first (under the lock) so the concurrent window is the
  # tests themselves, not compilation.
  step 04a-build cargo test --workspace --no-run
  stamp arbiter-launch
  bash "$here/measure-binary.sh" "$name-corpus" "$BINARY" corpus > "$out/04-arbiter.log" 2>&1 &
  arbiter=$!
  # Wait until the arbiter is analysing (its run directory exists).
  until [ -d "$private/$name-corpus" ] || ! kill -0 "$arbiter" 2>/dev/null; do sleep 1; done
  stamp arbiter-running
  stamp suite-start
  python3 scripts/run_test_binaries.py --log-dir "$private/tests-$name" > "$out/04-tests.log" 2>&1
  echo $? > "$out/04-tests.exit"
  stamp suite-end
  wait "$arbiter"
  echo $? > "$out/04-arbiter.exit"
  stamp arbiter-end
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
