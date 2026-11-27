#!/bin/sh
# tests.sh LOG_DIR -- CG23-RELATIONAL-POSTCONDITIONS: the gate's test-runner
# step alone, re-run after a disk-exhausted attempt; run under the shared
# heavy-work lock (flock). Exit captured to LOG_DIR/03-tests.exit; not piped.
set -u
root="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$root" || exit 2
logs="$root/$1"
mkdir -p "$logs"
CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 python3 scripts/run_test_binaries.py > "$logs/03-tests.log" 2>&1
echo $? > "$logs/03-tests.exit"
cat "$logs/03-tests.exit"
