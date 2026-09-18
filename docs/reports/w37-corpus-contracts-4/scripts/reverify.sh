#!/bin/sh
# Card W37-CORPUS-CONTRACTS-4: re-verify the one red row in ISOLATION.
#
# `a_declared_process_limit_stops_the_fixture_from_forking` is constraint 16's
# FIRST named load-sensitive row: it installs RLIMIT_NPROC=1, which counts every
# process of the real UID, so under load the host observes `Io(Broken pipe)`
# instead of `Terminated`. It failed once inside the full gate in exactly that
# documented shape. This runs it alone, N times, recording the load average
# before each repetition so the two facts can be reported together.
set -u
WT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$WT" || exit 99
L="$WT/measure/w37/logs"
OUT="$L/reverify.txt"
N=${1:-12}
: > "$OUT"
echo "cores: $(nproc)" >> "$OUT"
i=1
pass=0
fail=0
while [ "$i" -le "$N" ]; do
    echo "--- repetition $i ---" >> "$OUT"
    echo "load: $(uptime)" >> "$OUT"
    cargo test -p qcore-verifier-host --test enforced_process_limits -- \
        a_declared_process_limit_stops_the_fixture_from_forking \
        --exact --nocapture >> "$OUT" 2>&1
    code=$?
    echo "exit=$code" >> "$OUT"
    if [ "$code" -eq 0 ]; then pass=$((pass+1)); else fail=$((fail+1)); fi
    i=$((i+1))
done
echo "ISOLATED RESULT: $pass passed, $fail failed, of $N" >> "$OUT"
echo "ISOLATED RESULT: $pass passed, $fail failed, of $N"
