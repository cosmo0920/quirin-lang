#!/bin/sh
# CG16-INDEX-AND-OFFSET-EXTENTS: the rebased tree's `cargo check --workspace
# --all-targets`, then the full standing gate (scripts/gate.sh), under the
# integrator's shared heavy-step lock, then the dev profile cleaned (disk
# floor, wave 48 round 9). usage: gate-locked.sh LOCK LOG_DIR. Every exit is
# captured to a file; nothing is piped.
set -u
lock="$1"
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../../.." && pwd)"
cd "$root" || exit 2
logs="$root/$2"
mkdir -p "$logs"
CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 flock "$lock" sh -c '
  cargo check --workspace --all-targets > "$1/00-check.log" 2>&1
  echo $? > "$1/00-check.exit"
  sh "$2/gate.sh" "$3"
' gate "$logs" "$here" "$2"
cargo clean --profile dev > "$logs/11-clean.log" 2>&1
echo $? > "$logs/11-clean.exit"
