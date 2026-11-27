#!/usr/bin/env bash
# W47-LOOP-OWNER-COROUTINE-HOP: the standing gate (docs/agents/validation.md),
# every exit captured to its own file, logs private. No providers/ change, so
# the provider gate is not due; the provider cargo test runs once only to
# measure the matrix's provider_tests figure.
# usage: gate.sh LOG_DIR rust|records
#   rust:    submodule, Rust suite, clean, fmt, clippy, both probes, Lean
#            audits, clean, provider tests, provider clean
#   records: Python suite and docs-index check (after the matrix and index
#            regeneration), and the Clang recording
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$1"; phase="$2"
mkdir -p "$logs"
cd "$repo" || exit 2
export CARGO_INCREMENTAL=0
export PATH="$HOME/.elan/bin:$PATH"
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
case "$phase" in
  rust)
    step 01-submodule git submodule update --init --recursive
    step 02-fmt cargo fmt --all -- --check
    step 03-tests python3 scripts/run_test_binaries.py
    step 04-clean cargo clean --profile dev
    step 05-clippy cargo clippy --all-targets -- -D warnings
    step 06-probe cargo run -p qcore-compiler --example reevaluation_probe
    step 07-followup cargo run -p qcore-compiler --example reevaluation_followup
    step 08-lean python3 scripts/check_lean_audits.py
    step 09-clean cargo clean --profile dev
    step 17-provider-tests cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
    step 20-provider-clean cargo clean --manifest-path providers/Cargo.toml --profile dev
    ;;
  records)
    step 10-python python3 -m unittest discover -s scripts/tests
    step 11-docs-index python3 scripts/build_docs_index.py --check
    step 12-clang /usr/lib/llvm-21/bin/clang --version
    ;;
  *) echo "unknown phase $phase"; exit 2 ;;
esac
