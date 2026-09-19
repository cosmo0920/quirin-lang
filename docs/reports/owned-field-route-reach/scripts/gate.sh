#!/bin/bash
# OWNED-FIELD-ROUTE-REACH standing gate runner (AGENTS.md's, including the
# Python suite). Every step's exit is captured to its own file, never through a
# pipe. Run detached with `setsid -f` and poll `ALL_DONE`.
W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a941833997eae71fb
G="$W/measure/gate"
mkdir -p "$G"
cd "$W" || exit 99

git rev-parse HEAD > "$G/head.txt" 2>&1
df -h . > "$G/disk.txt" 2>&1

git submodule update --init --recursive > "$G/submodule.log" 2>&1
echo $? > "$G/submodule.exit"

cargo test --workspace --no-fail-fast > "$G/tests.log" 2>&1
echo $? > "$G/tests.exit"

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

python3 -m unittest discover -s scripts/tests > "$G/python.log" 2>&1
echo $? > "$G/python.exit"

# Run in THIS worktree, so the recording belongs to this run
# (`CLANG-RECORDING-UNBOUND-TO-ITS-RUN`: the generator cannot tell a borrowed
# recording from a real one, so the honesty is the card's).
clang --version > "$G/clang-version.txt" 2>&1
echo $? > "$G/clang-version.exit"

ls proof/Audit*.lean | wc -l > "$G/lean-audit-count.txt" 2>&1

echo done > "$G/ALL_DONE"
