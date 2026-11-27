#!/usr/bin/env bash
# gate.sh NAME [PART] -- RDHTTP-HEADER-SUMMARY-GAP (adapted from DECLARED-ASSUMPTION-BODY-CHECKS, VERDICT-BODY-STANDING-CLAUSE-SCOPE, VERDICT-CONTRACT-DEPENDENCIES, itself adapted from REFCOUNT-FAMILY-BLOCKING-CALLEES) (from CHUNKIO-META-LEN-BOUNDS-CONTRACTS, from
# CG10-INDIRECT-ACCESS-SOLE-BLOCKERS's scripts/gate.sh): cargo check and the
# standing per-change gate (docs/agents/validation.md), every step's output and
# exit to private/rhsg/gate-NAME/, nothing piped. PART is `code` (steps 00-07)
# or `docs` (08-09, after the records) or `all` (default). Step 04's suite run
# writes its logs to private/rhsg/tests-SUITE_NAME/ (default gate-NAME; the first
# gate, g1, wrote tests-a3) and its wall time (inside the lock)
# to private/rhsg/tests-SUITE_NAME.seconds (g1: the third AFTER suite timing run).
# Each step runs under the shared heavy lock.
set -u
name="$1"
part="${2:-all}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/rhsg/gate-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  flock "$lock" "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
if [ "$part" != docs ]; then
  # The caller records HEAD into $out/head and runs step 01 (the submodule
  # update) itself before this script, its exit in $out/01-submodule.exit.
  step 00-check cargo check --workspace --all-targets
  step 02-fmt cargo fmt --all -- --check
  step 03-clippy cargo clippy --locked --all-targets -- -D warnings
  flock "$lock" /usr/bin/env bash -c '
    start=$EPOCHREALTIME
    python3 scripts/run_test_binaries.py --log-dir "$1/tests-$3" > "$2/04-tests.log" 2>&1
    rc=$?
    end=$EPOCHREALTIME
    echo "$rc" > "$2/04-tests.exit"
    echo "$rc" > "$1/tests-$3.exit"
    awk -v s="$start" -v e="$end" "BEGIN { printf \"%.3f\n\", e - s }" > "$1/tests-$3.seconds"
  ' _ "$repo/private/rhsg" "$out" "${SUITE_NAME:-gate-$name}"
  step 05-probe cargo run -p qcore-compiler --example reevaluation_probe
  step 06-followup cargo run -p qcore-compiler --example reevaluation_followup
  step 07-lean python3 scripts/check_lean_audits.py
  clang --version > "$out/clang-version.txt" 2>&1
  echo $? > "$out/clang-version.exit"
  echo done > "$out/code-done"
fi
if [ "$part" != code ]; then
  # The caller records HEAD into $out/docs-head.
  step 08-python python3 -m unittest discover -s scripts/tests
  step 09-index python3 scripts/build_docs_index.py --check
  echo done > "$out/docs-done"
fi
