#!/bin/sh
# Card C-GATEWAY-INDIRECT-CENSUS-2: the standing per-change gate from AGENTS.md,
# run in order, with every exit code captured to a file (never through a pipe).
# A nonzero probe exit is a finding, not a build failure.
#
# Every step runs with the `indirect-census` feature OFF, which is how the
# workspace builds it: the feature is declared on qcore-c-gateway and enabled by
# no workspace member. The census code is therefore not in any binary the gate
# measures. A separate clippy pass over the crate WITH the feature on is run
# beside it, so the instrumentation is linted rather than merely unlinted.
set -u
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a1204a1a083d35165
OUT="$WT/measure/c-gateway-indirect-census-2/gate"
mkdir -p "$OUT"
cd "$WT" || exit 99

run() {
    name=$1
    shift
    echo "=== $name ==="
    "$@" > "$OUT/$name.log" 2>&1
    code=$?
    echo "$name exit=$code"
    echo "$name exit=$code" >> "$OUT/exits.txt"
}

: > "$OUT/exits.txt"
run submodule       git submodule update --init --recursive
run workspace_tests cargo test --workspace --no-fail-fast
run fmt             cargo fmt --all -- --check
run clippy          cargo clippy --all-targets -- -D warnings
run clippy_census   cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -- -D warnings
run probe           cargo run -p qcore-compiler --example reevaluation_probe
run followup        cargo run -p qcore-compiler --example reevaluation_followup
run lean_audits     python3 scripts/check_lean_audits.py

echo
echo "=== exits ==="
cat "$OUT/exits.txt"
echo
echo "=== workspace test totals ==="
grep -E "^test result:" "$OUT/workspace_tests.log" \
  | awk '{p+=$4; f+=$6; i+=$8} END {printf "passed=%d failed=%d ignored=%d summaries=%d\n", p, f, i, NR}'
echo "=== failures, if any ==="
grep -E "^(failures:|test .* FAILED|error(\[|:))" "$OUT/workspace_tests.log" | head -40
echo "=== lean ==="
tail -3 "$OUT/lean_audits.log"
