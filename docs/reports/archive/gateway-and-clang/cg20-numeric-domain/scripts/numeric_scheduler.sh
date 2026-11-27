#!/usr/bin/env bash
# numeric_scheduler.sh -- CG20-NUMERIC-DOMAIN: the arithmetic property for
# Fluent Bit's scheduler backoff (W45-FB-SCHED-RANDOM-RANGE), with the prepared
# tail-pause attempt's recorded compile command, into private/cg20/numeric/.
# Every exit is written to a file; nothing is piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg20/numeric"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
database=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-production/compile_commands.json
for name in random_uniform backoff_full_jitter; do
  CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 cargo run -q -p qcore-c-gateway --example numeric_obligations -- \
    "$database" src/flb_scheduler.c "$name" > "$out/$name.json" 2> "$out/$name.err"
  echo $? > "$out/$name.exit"
done
python3 "$here/summarize_numeric.py" "$out/random_uniform.json" "$out/backoff_full_jitter.json" \
  > "$out/summary.txt" 2>&1
echo $? > "$out/summary.exit"
