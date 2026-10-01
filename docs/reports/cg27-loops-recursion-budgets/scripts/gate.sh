#!/usr/bin/env bash
# gate.sh STEP... -- CG27-LOOPS-RECURSION-BUDGETS: the standing gate
# (docs/agents/validation.md), one step at a time, each exit written to
# private/cg27lrb-gate/STEP.exit and its output to STEP.log. Nothing is piped.
# Steps: submodule tests fmt clippy probe followup lean python docs
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg27lrb-gate"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
for step in "$@"; do
  case "$step" in
    submodule) git submodule update --init --recursive ;;
    tests) python3 scripts/run_test_binaries.py --jobs 4 ;;
    fmt) cargo fmt --all -- --check ;;
    clippy) cargo clippy --locked --all-targets -- -D warnings ;;
    probe) cargo run -p qcore-compiler --example reevaluation_probe ;;
    followup) cargo run -p qcore-compiler --example reevaluation_followup ;;
    lean) python3 scripts/check_lean_audits.py ;;
    python) python3 -m unittest discover -s scripts/tests ;;
    docs) python3 scripts/build_docs_index.py --check ;;
    *) echo "unknown step $step"; exit 2 ;;
  esac > "$out/$step.log" 2>&1
  echo $? > "$out/$step.exit"
done
