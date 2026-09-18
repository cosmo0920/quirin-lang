#!/bin/bash
# CT42-CELL-ROUTE full standing gate, run detached (setsid -f, no trap '' TERM).
# Every exit lands in its own file; no pipe carries a step's exit
# (GATE-EXIT-SWALLOWED-BY-PIPE). Usage: gate.sh <worktree> <output-dir>
W=${1:?worktree}
O=${2:?output dir}
mkdir -p "$O"
cd "$W" || exit 90
date -u +%FT%TZ > "$O/start.txt"
git rev-parse HEAD > "$O/head.txt"
git status --short > "$O/status.txt"
df -BG . > "$O/df-start.txt"

CLANG_BIN="${QCORE_CLANG:-clang}"
"$CLANG_BIN" --version > "$O/clang-version.txt" 2>&1
echo $? > "$O/clang-version.exit"
echo "QCORE_CLANG=${QCORE_CLANG:-<unset>}" >> "$O/clang-version.txt"

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

PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py > "$O/lean.log" 2>&1
echo $? > "$O/lean.exit"

python3 -m unittest discover -s scripts/tests > "$O/python.log" 2>&1
echo $? > "$O/python.exit"

df -BG . > "$O/df-end.txt"
date -u +%FT%TZ > "$O/end.txt"
echo done > "$O/DONE"
