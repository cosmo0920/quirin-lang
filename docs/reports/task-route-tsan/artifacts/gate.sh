#!/bin/sh
# Standing per-change gate (AGENTS.md). Run-scoped output directory; every
# step's exit captured to its own file with no pipe in the command.
set -u
out="$1"
mkdir -p "$out"

cargo test --workspace --no-fail-fast > "$out/test.log" 2>&1
echo "$?" > "$out/test.rc"

cargo fmt --all -- --check > "$out/fmt.log" 2>&1
echo "$?" > "$out/fmt.rc"

cargo clippy --all-targets -- -D warnings > "$out/clippy.log" 2>&1
echo "$?" > "$out/clippy.rc"

cargo run -p qcore-compiler --example reevaluation_probe > "$out/probe.log" 2>&1
echo "$?" > "$out/probe.rc"

cargo run -p qcore-compiler --example reevaluation_followup > "$out/followup.log" 2>&1
echo "$?" > "$out/followup.rc"

python3 scripts/check_lean_audits.py > "$out/lean.log" 2>&1
echo "$?" > "$out/lean.rc"

echo done > "$out/COMPLETE"
