#!/usr/bin/env bash
# gate.sh OUT [STEP...] -- CG29-EXECUTION-COVERAGE-AND-RECLAMATION's standing gate
# (docs/agents/validation.md), every step's output and exit to OUT, nothing
# piped. With STEP arguments only those steps run (01 02 03 05 06 07 08 09 10).
set -u
out="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
steps="${*:-01 02 03 05 06 07 08 09 10}"
run() {
  local id="$1"; shift
  case " $steps " in *" $id "*) ;; *) return ;; esac
  "$@" > "$out/$id.log" 2>&1
  echo $? > "$out/$id.exit"
}
run 01 git submodule update --init --recursive
run 02 cargo fmt --all -- --check
run 03 python3 scripts/run_test_binaries.py
run 05 cargo clippy --locked --all-targets -- -D warnings
run 06 cargo run -p qcore-compiler --example reevaluation_probe
run 07 cargo run -p qcore-compiler --example reevaluation_followup
run 08 python3 scripts/check_lean_audits.py
run 09 python3 -m unittest discover -s scripts/tests
run 10 python3 scripts/build_docs_index.py --check
clang --version > "$out/clang-version.txt" 2>&1
