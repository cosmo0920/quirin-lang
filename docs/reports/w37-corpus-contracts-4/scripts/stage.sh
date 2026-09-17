#!/bin/sh
# Card W37-CORPUS-CONTRACTS-4: run one audit stage, capturing the exit code to
# a file rather than through a pipe.
set -u
WT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$WT" || exit 99
STAGE="$1"
BIN="$WT/target/release/examples/cycle19_fluent_bit_libraries_audit"
L="$WT/measure/w37/logs"
mkdir -p "$L"
python3 -B "$WT/measure/w37/run_audit.py" "$STAGE" "$BIN" > "$L/$STAGE-run.txt" 2>&1
echo "$?" > "$L/$STAGE-exit.txt"
