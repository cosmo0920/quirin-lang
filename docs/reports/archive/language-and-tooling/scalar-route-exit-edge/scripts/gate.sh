#!/bin/bash
# The standing per-change gate. Every exit captured to its own file, no pipes.
set -u
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a169a5e9421edcb6d
G=$WT/measure/scalar-route-exit-edge/gate
mkdir -p "$G"
cd "$WT" || exit 90

git submodule update --init --recursive > "$G/submodule.log" 2>&1
echo $? > "$G/submodule.exit"

cargo test --workspace --no-fail-fast > "$G/test.log" 2>&1
echo $? > "$G/test.exit"

cargo fmt --all -- --check > "$G/fmt.log" 2>&1
echo $? > "$G/fmt.exit"

cargo clippy --all-targets -- -D warnings > "$G/clippy.log" 2>&1
echo $? > "$G/clippy.exit"

cargo run -p qcore-compiler --example reevaluation_probe > "$G/probe.log" 2>&1
echo $? > "$G/probe.exit"

cargo run -p qcore-compiler --example reevaluation_followup > "$G/followup.log" 2>&1
echo $? > "$G/followup.exit"

python3 scripts/check_lean_audits.py > "$G/lean.log" 2>&1
echo $? > "$G/lean.exit"

echo DONE > "$G/gate.done"
