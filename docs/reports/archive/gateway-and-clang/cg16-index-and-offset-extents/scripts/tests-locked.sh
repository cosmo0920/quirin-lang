#!/bin/sh
# CG16-INDEX-AND-OFFSET-EXTENTS: the gate's workspace test step alone, under the
# integrator's shared heavy-step lock (disk floor, wave 48 round 9), then the
# dev profile cleaned. usage: tests-locked.sh LOCK LOG_DIR (LOG_DIR relative to
# the checkout root). The exit is captured to a file; nothing is piped.
set -u
lock="$1"
root="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$root" || exit 2
logs="$root/$2"
mkdir -p "$logs"
CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 flock "$lock" python3 scripts/run_test_binaries.py > "$logs/03-tests.log" 2>&1
echo $? > "$logs/03-tests.exit"
cargo clean --profile dev > "$logs/03-clean.log" 2>&1
echo $? > "$logs/03-clean.exit"
