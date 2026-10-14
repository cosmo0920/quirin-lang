#!/usr/bin/env bash
# targeted.sh NAME -- SERVE-FORWARDED-CALLBACK-FORMALS: the integrator's
# targeted check after the rebase onto 89907068d (OP-CALLBACK-CANDIDATE-
# CONTRACTS), run as `flock <heavy.lock> bash targeted.sh NAME`: cargo check of
# the workspace, the gateway crate's tests, the compiler's contract tests, the
# Lean audits. Every step's output and exit go to private/sf/targeted-NAME/;
# nothing is piped. The Python suite and the docs index run after the matrix
# regeneration (finish-targeted.sh).
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/sf/targeted-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
git rev-parse HEAD > "$out/head"
step 00-check cargo check --workspace --all-targets
step 01-gateway cargo test -p qcore-c-gateway --no-fail-fast
step 02-contract-loader cargo test -p qcore-compiler --test contract_loader
step 03-contract-corpus cargo test -p qcore-compiler --test qcore_contract_corpus
step 04-view-census cargo test -p qcore-compiler --test contract_view_census
step 05-lean python3 scripts/check_lean_audits.py
echo done > "$out/done"
