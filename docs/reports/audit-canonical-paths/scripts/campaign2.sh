#!/bin/sh
# One lock hold: build each side's snapshot at the fixed path, then run every
# route (run_pinned_paths.sh). Usage: campaign2.sh WORKTREE BASE_REV TIP_REV
set -u
W=$1
A=$W/private/acp
S=$W/docs/reports/audit-canonical-paths/scripts
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0
for side in base-mirror tip-mirror tip-original base-namespace; do
  date -u +%Y-%m-%dT%H:%M:%SZ >"$A/builds/$side.start"
  unshare -rm sh "$S/pin_run.sh" "$A/snap-$side" "$A/snap" "$A/builds" "$A/out" no \
    cargo build --release -p qcore-audit --bin quirin-audit >"$A/builds/$side.log" 2>&1
  echo $? >"$A/builds/$side.exit"
  sha256sum "$A/snap-$side/target/release/quirin-audit" >"$A/builds/$side.sha256"
  [ "$(cat "$A/builds/$side.exit")" = 0 ] || exit 8
done
sh "$S/run_pinned_paths.sh" "$W" "$2" "$3" runs2
