#!/usr/bin/env bash
# W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL: the standing gate
# (docs/agents/validation.md); every exit captured to its own file, logs
# private. No provider gate: providers/ is unchanged. The submodule step
# (`git submodule update --init --recursive`) is run by hand before this
# script, its exit recorded as 01-submodule.exit in LOG_DIR.
# usage: gate.sh LOG_DIR rust|records
#   rust:    fmt, Rust suite, clean, clippy, both probes, Lean audits, clean
#   records: Python suite and docs-index check (after the matrix and index
#            regeneration), and the Clang recording
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$1"; phase="$2"
mkdir -p "$logs"
cd "$repo" || exit 2
export CARGO_INCREMENTAL=0
export PATH="/home/cosmo/.elan/bin:$PATH"
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
case "$phase" in
  rust)
    step 02-fmt cargo fmt --all -- --check
    step 03-tests python3 scripts/run_test_binaries.py
    step 04-clean cargo clean --profile dev
    step 05-clippy cargo clippy --all-targets -- -D warnings
    step 06-probe cargo run -p qcore-compiler --example reevaluation_probe
    step 07-followup cargo run -p qcore-compiler --example reevaluation_followup
    step 08-lean python3 scripts/check_lean_audits.py
    step 09-clean cargo clean --profile dev
    ;;
  records)
    step 10-python python3 -m unittest discover -s scripts/tests
    step 11-docs-index python3 scripts/build_docs_index.py --check
    step 12-clang /usr/lib/llvm-21/bin/clang --version
    ;;
  *) echo "unknown phase $phase"; exit 2 ;;
esac
