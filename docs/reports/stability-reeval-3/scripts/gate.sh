#!/bin/bash
# STABILITY-REEVAL-3 full standing gate, run detached. Every exit lands in
# its own file; no pipe carries a step's exit (GATE-EXIT-SWALLOWED-BY-PIPE).
W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ac5c249e32fa73606
O=$W/private/stability-reeval-3/gate-base
mkdir -p "$O"
cd "$W" || exit 90
date -u +%FT%TZ > "$O/start.txt"
git rev-parse HEAD > "$O/head.txt"
df -BG . > "$O/df-start.txt"

cargo test --workspace --no-fail-fast > "$O/cargo-test.log" 2>&1
echo $? > "$O/cargo-test.exit"
date -u +%FT%TZ > "$O/cargo-test-end.txt"

cargo fmt --all -- --check > "$O/fmt.log" 2>&1
echo $? > "$O/fmt.exit"

cargo clippy --all-targets -- -D warnings > "$O/clippy.log" 2>&1
echo $? > "$O/clippy.exit"

cargo run -p qcore-compiler --example reevaluation_probe > "$O/probe.log" 2>&1
echo $? > "$O/probe.exit"

cargo run -p qcore-compiler --example reevaluation_followup > "$O/followup.log" 2>&1
echo $? > "$O/followup.exit"

python3 scripts/check_lean_audits.py > "$O/lean.log" 2>&1
echo $? > "$O/lean.exit"

python3 -m unittest discover -s scripts/tests > "$O/python.log" 2>&1
echo $? > "$O/python.exit"

df -BG . > "$O/df-end.txt"
date -u +%FT%TZ > "$O/end.txt"
echo done > "$O/DONE"
