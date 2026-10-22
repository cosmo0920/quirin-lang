#!/bin/sh
# One locked job for W48-SCHED-CLEANUP-PRODUCTION (run under
# `flock /tmp/qcore-c-card-validation-20261022.lock`): TIP release build, TIP
# snapshot, TIP corpus route, then the full standing gate. Every step's exit is
# captured to a file, never piped. Usage: tip_job.sh RUNDIR
set -u
run=$1
here=$(pwd)
mkdir -p "$run" || exit 2
# A disk-exhausted run measures disk, not the program: refuse below 40 GiB.
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
export CARGO_BUILD_JOBS=6
git rev-parse HEAD >"$run/head"
git status --porcelain >"$run/status-before"
step tip-build cargo build --release -p qcore-audit --bin quirin-audit
sha256sum target/release/quirin-audit >"$run/tip-binary.sha256"
snapshot="$run/tip-snapshot"
mkdir -p "$snapshot/target/release"
git archive HEAD | tar -x -C "$snapshot"
cp target/release/quirin-audit "$snapshot/target/release/quirin-audit"
step tip-corpus python3 docs/reports/w48-sched-cleanup-production/scripts/measure.py corpus "$run/tip-corpus-8" \
  --link-map docs/reports/w48-sched-cleanup-production/link-map.corpus.json \
  --inputs-snapshot "$snapshot" --binary-source "$(cat "$run/head")" --jobs 4
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
cd "$here"
echo done >"$run/job.done"
