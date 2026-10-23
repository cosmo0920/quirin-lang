#!/bin/sh
# The resumed card's corpus pair (run under
# `flock /tmp/qcore-c-card-validation-20261022.lock`): BASE = BASE_REV
# (4dd8c8013), TIP = this branch's HEAD at start. Same route as
# merged_job.sh without its gate, which runs separately (gate_job.sh) on the
# final commit; incremental compilation off.
# Both release binaries are built in this worktree (BASE by a detached
# checkout, restored before TIP), so each binary's compile-time root is the
# worktree, whose Libevent submodule both pin identically. Each is copied
# into a `git archive` snapshot of its own revision; both corpus routes run
# concurrently at --jobs 4 each.
# Every step's exit is captured to a file, never piped.
# Usage: pair_job.sh RUNDIR BASE_REV BRANCH
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
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0
tip_rev=$(git rev-parse HEAD)
echo "$tip_rev" >"$run/tip-head"
git rev-parse "$base_rev" >"$run/base-head"
git status --porcelain >"$run/status-before"
# BASE binary and snapshot.
step base-checkout git checkout --detach "$base_rev"
step base-build cargo build --release -p qcore-audit --bin quirin-audit
sha256sum target/release/quirin-audit >"$run/base-binary.sha256"
mkdir -p "$run/base-snapshot/target/release"
git archive "$base_rev" | tar -x -C "$run/base-snapshot"
cp target/release/quirin-audit "$run/base-snapshot/target/release/quirin-audit"
step tip-checkout git checkout "$branch"
git rev-parse HEAD >"$run/tip-head-restored"
# TIP binary and snapshot.
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
git rev-parse HEAD >"$run/head-after"
git status --porcelain >"$run/status-after"
df --output=avail -B1 . | tail -1 >"$run/free-bytes-after"
echo done >"$run/job.done"
