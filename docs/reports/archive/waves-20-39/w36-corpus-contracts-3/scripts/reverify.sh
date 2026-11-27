#!/bin/sh
# Card W36-CORPUS-CONTRACTS-3: re-verify the one red row in ISOLATION.
#
# `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
# failed once inside the full gate on a wall-clock assertion
# (`start.elapsed() < Duration::from_secs(1)`, standalone_host.rs:175). This
# runs it alone, N times, recording the load average before each repetition so
# the two facts can be reported together.
set -u
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a52a81a2f3ecff0f0
cd "$WT" || exit 99
L="$WT/measure/w36/logs"
OUT="$L/reverify.txt"
N=${1:-10}
: > "$OUT"
echo "cores: $(nproc)" >> "$OUT"
i=1
pass=0
fail=0
while [ "$i" -le "$N" ]; do
    echo "--- repetition $i ---" >> "$OUT"
    echo "load: $(uptime)" >> "$OUT"
    cargo test -p qcore-verifier-host --test standalone_host -- \
        closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request \
        --exact --nocapture >> "$OUT" 2>&1
    code=$?
    echo "exit=$code" >> "$OUT"
    if [ "$code" -eq 0 ]; then pass=$((pass+1)); else fail=$((fail+1)); fi
    i=$((i+1))
done
echo "ISOLATED RESULT: $pass passed, $fail failed, of $N" >> "$OUT"
echo "ISOLATED RESULT: $pass passed, $fail failed, of $N"
