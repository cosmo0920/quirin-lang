#!/bin/sh
# Card W48-QR-FLOAT-IEEE-LEAN-MODEL: time `python3 scripts/check_lean_audits.py`.
# Usage (from the repository root): docs/reports/w48-qr-float-ieee-lean/time_lean_audits.sh OUT_DIR [cold]
# With `cold`, proof/.lake/build (this checkout's own build output) is removed
# first, so the run rebuilds every module. Writes OUT_DIR/{stdout.log,stderr.log,exit,wall-seconds}.
set -u
OUT="$1"
mkdir -p "$OUT"
if [ "${2:-}" = cold ]; then
  rm -rf proof/.lake/build
fi
start=$(date +%s.%N)
python3 scripts/check_lean_audits.py > "$OUT/stdout.log" 2> "$OUT/stderr.log"
echo $? > "$OUT/exit"
end=$(date +%s.%N)
echo "$start $end" | awk '{printf "%.2f\n", $2-$1}' > "$OUT/wall-seconds"
