#!/usr/bin/env bash
# CG21-CALL-EXITS-AND-BOUNDARIES: the gate's test step alone, for a shared
# host. Run it under the coordinator's lock:
#   flock HEAVY_LOCK bash tests-locked.sh LOG_DIR
# Inside the lock it refuses to start below 40 GiB free (exit 3, recorded),
# runs scripts/run_test_binaries.py, then `cargo clean --profile dev`. Every
# exit is captured to LOG_DIR/<step>.exit.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../../.." && pwd)"
cd "$root" || exit 2
logs="$root/$1"
mkdir -p "$logs"
df -h / > "$logs/03-tests.disk-before.txt" 2>&1
avail=$(df --output=avail -BG / | tail -1 | tr -dc 0-9)
if [ "$avail" -lt 40 ]; then
  echo "refused: ${avail}G free, below the 40 GiB floor" > "$logs/03-tests.log"
  echo 3 > "$logs/03-tests.exit"
  exit 3
fi
CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 python3 scripts/run_test_binaries.py > "$logs/03-tests.log" 2>&1
echo $? > "$logs/03-tests.exit"
df -h / > "$logs/03-tests.disk-after.txt" 2>&1
cargo clean --profile dev > "$logs/04-clean.log" 2>&1
echo $? > "$logs/04-clean.exit"
