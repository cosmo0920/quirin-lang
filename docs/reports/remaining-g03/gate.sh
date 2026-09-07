#!/bin/sh
# G03 standing per-change gate. Writes one combined log; each section records
# its own exit code so a nonzero probe exit is visible as a finding rather
# than hidden by a later success.
set -u
PATH="$HOME/.elan/bin:$PATH"
export PATH
LOG="docs/reports/remaining-g03/gate.log"
: > "$LOG"

section() {
  name="$1"
  shift
  printf '\n===== %s =====\n' "$name" >> "$LOG"
  "$@" >> "$LOG" 2>&1
  status=$?
  printf -- '----- %s exit=%s -----\n' "$name" "$status" >> "$LOG"
}

section "cargo test --workspace" cargo test --workspace
section "cargo fmt --all -- --check" cargo fmt --all -- --check
section "cargo clippy --all-targets -- -D warnings" cargo clippy --all-targets -- -D warnings
section "reevaluation_probe" cargo run -p qcore-compiler --example reevaluation_probe
section "reevaluation_followup" cargo run -p qcore-compiler --example reevaluation_followup
section "provider tests" sh -c 'cd providers && cargo test --workspace'
section "python trace tests" python3 -B -m unittest discover -s scripts/tests -t scripts/tests -v
section "lean audits" python3 -B scripts/check_lean_audits.py
section "lake build" sh -c 'cd proof && lake build'
section "chunkio-core audit" cargo run --quiet -p qcore-audit -- contract/profiles/chunkio-core/audit.json
section "cmetrics-counter audit" cargo run --quiet -p qcore-audit -- contract/profiles/cmetrics-counter/audit.json
section "g03 fail-closed mutations" python3 -B docs/reports/remaining-g03/failclosed.py
printf '\ngate finished\n' >> "$LOG"
