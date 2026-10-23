#!/bin/sh
# The resumed card's closing evidence on its final commit, as one locked job
# (run under `flock /tmp/qcore-c-card-validation-20261022.lock`): the full
# standing gate (gate_job.sh), the provider gate (provider_gate.sh) and
# `pin --check` of the Monkey provider at both audited revisions with the
# BASE and TIP release binaries of the pair run (pin_check.sh). Each step's
# exit is captured to a file by the called script, never piped.
# Usage: final_gate_job.sh RUNDIR PAIR_RUNDIR
set -u
run=$1
pair=$2
scripts=docs/reports/w48-sched-cleanup-production/scripts
mkdir -p "$run" || exit 2
git rev-parse HEAD >"$run/head"
sh "$scripts/gate_job.sh" "$run/gate"
echo $? >"$run/gate-job.exit"
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  sh "$scripts/provider_gate.sh" "$run/provider"
echo $? >"$run/provider-job.exit"
sh "$scripts/pin_check.sh" "$pair/base-snapshot/target/release/quirin-audit" "$pair/base-snapshot" "$run/pin-base"
sh "$scripts/pin_check.sh" "$pair/tip-snapshot/target/release/quirin-audit" "$pair/tip-snapshot" "$run/pin-tip"
git rev-parse HEAD >"$run/head-after"
git status --porcelain >"$run/status-after"
echo done >"$run/job.done"
