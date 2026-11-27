#!/bin/sh
# CFG-RELEASE: build each side's git-archive snapshot at one fixed path, inside a
# private mount namespace (AUDIT-CANONICAL-PATHS pin_run.sh), so both release
# binaries embed the same paths. Both builds use the fixed comparison label
# QCORE_AUDIT_BUILD_COMMIT=BASE; each side's actual source and binary SHA-256
# are recorded separately. Builds take no gate slot (docs/agents/validation.md).
# Usage: build_sides.sh WORKTREE BASE_REV LOGDIR
set -u
W=$1
BASE_REV=$2
LOG=$3
A=$W/private/cfr
PIN=$W/docs/reports/audit-canonical-paths/scripts/pin_run.sh
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 QCORE_AUDIT_BUILD_COMMIT=$BASE_REV
mkdir -p "$LOG"
for side in base tip; do
  date -u +%Y-%m-%dT%H:%M:%SZ >"$LOG/build-$side.start"
  unshare -rm sh "$PIN" "$A/snap-$side" "$A/snap" "$A/builds" "$A/out" no \
    cargo build --release -p qcore-audit --bin quirin-audit >"$LOG/build-$side.log" 2>&1
  echo $? >"$LOG/build-$side.exit"
  sha256sum "$A/snap-$side/target/release/quirin-audit" >"$LOG/build-$side.sha256"
  [ "$(cat "$LOG/build-$side.exit")" = 0 ] || exit 8
done
