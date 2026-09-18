#!/bin/bash
# STABILITY-REEVAL-4 standing gate runner. Exits captured to files, never through a pipe.
W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a501978ba40d1cc3b
G="$W/measure/gate"
mkdir -p "$G"
cd "$W" || exit 99

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

clang --version > "$G/clang-version.txt" 2>&1
echo $? > "$G/clang-version.exit"

echo done > "$G/ALL_DONE"
