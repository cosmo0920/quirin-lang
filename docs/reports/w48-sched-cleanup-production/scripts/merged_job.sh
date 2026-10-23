#!/bin/sh
# One locked job for W48-SCHED-CLEANUP-PRODUCTION on the integrator's main
# (run under `flock /tmp/qcore-c-card-validation-20261022.lock`):
#   BASE2 = main BASE_REV, TIP2 = this branch's merge of it (HEAD at start).
# Both release binaries are built in this worktree (BASE2 by a detached
# checkout, restored before TIP2), so each binary's compile-time root is the
# worktree, whose Libevent submodule both pin identically. Each is copied
# into a `git archive` snapshot of its own revision; both corpus routes run
# concurrently at --jobs 4 each, then the full standing gate runs on TIP2.
# Every step's exit is captured to a file, never piped.
# Usage: merged_job.sh RUNDIR BASE_REV BRANCH
set -u
run=$1
base_rev=$2
branch=$3
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
export CARGO_BUILD_JOBS=6
tip_rev=$(git rev-parse HEAD)
echo "$tip_rev" >"$run/tip-head"
git rev-parse "$base_rev" >"$run/base-head"
git status --porcelain >"$run/status-before"
# BASE2 binary and snapshot.
step base-checkout git checkout --detach "$base_rev"
step base-build cargo build --release -p qcore-audit --bin quirin-audit
sha256sum target/release/quirin-audit >"$run/base-binary.sha256"
mkdir -p "$run/base-snapshot/target/release"
git archive "$base_rev" | tar -x -C "$run/base-snapshot"
cp target/release/quirin-audit "$run/base-snapshot/target/release/quirin-audit"
step tip-checkout git checkout "$branch"
git rev-parse HEAD >"$run/tip-head-restored"
# TIP2 binary and snapshot.
step tip-build cargo build --release -p qcore-audit --bin quirin-audit
sha256sum target/release/quirin-audit >"$run/tip-binary.sha256"
mkdir -p "$run/tip-snapshot/target/release"
git archive "$tip_rev" | tar -x -C "$run/tip-snapshot"
cp target/release/quirin-audit "$run/tip-snapshot/target/release/quirin-audit"
# Both corpus routes, concurrently.
measure=docs/reports/w48-sched-cleanup-production/scripts/measure.py
map=docs/reports/w48-sched-cleanup-production/link-map.corpus.json
date -u +%Y-%m-%dT%H:%M:%SZ >"$run/corpus.start"
python3 "$measure" corpus "$run/base-corpus" --link-map "$map" --inputs-snapshot "$run/base-snapshot" \
  --binary-source "$(cat "$run/base-head")" --jobs 4 >"$run/base-corpus.log" 2>&1 &
base_pid=$!
python3 "$measure" corpus "$run/tip-corpus" --link-map "$map" --inputs-snapshot "$run/tip-snapshot" \
  --binary-source "$tip_rev" --jobs 4 >"$run/tip-corpus.log" 2>&1 &
tip_pid=$!
echo "$base_pid $tip_pid" >"$run/corpus.pids"
wait "$base_pid"
echo $? >"$run/base-corpus.exit"
wait "$tip_pid"
echo $? >"$run/tip-corpus.exit"
date -u +%Y-%m-%dT%H:%M:%SZ >"$run/corpus.end"
# The standing gate on TIP2.
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
