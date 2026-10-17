#!/usr/bin/env bash
# Full standing gate for W44. Usage: gate.sh RUN_NAME
set -u
name="${1:?run name required}"
repo="$(cd "$(dirname "$0")/../../../.." && pwd)"
out="$repo/private/w44xu/gate-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_TARGET_DIR=/home/cosmo/Documents/GitHub/qcore-c/private/w44xu-target

run_step() {
  local label="$1"
  shift
  "$@" > "$out/$label.log" 2>&1
  local status=$?
  printf '%s\n' "$status" > "$out/$label.exit"
  printf '%s %s\n' "$label" "$status" >> "$out/exits.txt"
}

: > "$out/exits.txt"
git rev-parse HEAD > "$out/head.txt"
run_step submodule git submodule update --init --recursive
run_step rust-tests python3 scripts/run_test_binaries.py
run_step fmt cargo fmt --all -- --check
run_step clippy cargo clippy --all-targets -- -D warnings
run_step reevaluation-probe cargo run -p qcore-compiler --example reevaluation_probe
run_step reevaluation-followup cargo run -p qcore-compiler --example reevaluation_followup
run_step lean-audits python3 scripts/check_lean_audits.py
run_step python-tests python3 -m unittest discover -s scripts/tests

if awk '$2 != 0 { bad = 1 } END { exit bad ? 0 : 1 }' "$out/exits.txt"; then
  exit 1
fi
