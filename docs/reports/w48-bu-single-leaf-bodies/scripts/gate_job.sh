#!/bin/sh
# The full standing gate for W48-BU-SINGLE-LEAF-BODIES (run under
# `flock /tmp/qcore-c-card-validation-20261022.lock`), with incremental
# compilation and debug info off to bound disk. Every step's exit is captured
# to a file, never piped. Usage: gate_job.sh RUNDIR
set -u
run=$1
mkdir -p "$run" || exit 2
free=$(df --output=avail -B1 . | tail -1)
echo "$free" >"$run/free-bytes-before" || exit 2
if [ "$free" -lt 42949672960 ]; then
  echo "refused: $free bytes free" >"$run/job.refused"
  exit 3
fi
step() {
  name=$1
  shift
  date -u +%Y-%m-%dT%H:%M:%SZ >"$run/$name.start"
  "$@" >"$run/$name.log" 2>&1
  echo $? >"$run/$name.exit"
  date -u +%Y-%m-%dT%H:%M:%SZ >"$run/$name.end"
}
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
git rev-parse HEAD >"$run/head"
git status --porcelain >"$run/status-before"
step gate-submodule git submodule update --init --recursive
step gate-rust python3 scripts/run_test_binaries.py
step gate-fmt cargo fmt --all -- --check
step gate-clippy cargo clippy --all-targets -- -D warnings
step gate-probe cargo run -p qcore-compiler --example reevaluation_probe
step gate-followup cargo run -p qcore-compiler --example reevaluation_followup
step gate-lean env PATH="/home/cosmo/.elan/bin:$PATH" python3 scripts/check_lean_audits.py
step gate-python python3 -m unittest discover -s scripts/tests
clang --version >"$run/clang-version.txt" 2>&1
echo $? >"$run/clang-version.exit"
git rev-parse HEAD >"$run/head-after"
git status --porcelain >"$run/status-after"
df --output=avail -B1 . | tail -1 >"$run/free-bytes-after"
echo done >"$run/job.done"
