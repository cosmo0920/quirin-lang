#!/bin/sh
# Card W37-CORPUS-CONTRACTS-4: the standing per-change gate from AGENTS.md.
# Every step's exit code is captured to a file, never read through a pipe.
set -u
WT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$WT" || exit 99
L="$WT/measure/w37/logs"
mkdir -p "$L"
E="$L/gate-exits.txt"
: > "$E"

run() {
    name="$1"
    shift
    "$@" > "$L/gate-$name.log" 2>&1
    code=$?
    echo "$name=$code" >> "$E"
}

run test cargo test --workspace --no-fail-fast
run fmt cargo fmt --all -- --check
run clippy cargo clippy --all-targets -- -D warnings
run probe cargo run -p qcore-compiler --example reevaluation_probe
run followup cargo run -p qcore-compiler --example reevaluation_followup
PATH="$HOME/.elan/bin:$PATH" run lean python3 scripts/check_lean_audits.py
run pytests python3 -m unittest discover -s scripts/tests

echo "GATE_DONE" >> "$E"
