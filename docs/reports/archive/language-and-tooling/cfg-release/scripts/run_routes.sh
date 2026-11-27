#!/bin/sh
# CFG-RELEASE identity and memory run: BASE and TIP on the original pinned
# inputs, corpus, slice and system routes. Each side has its own git-archive
# snapshot (pinned libevent cloned in), built at the fixed path by
# build_sides.sh; each route of each side runs in an `audit` slot of
# scripts/gate_slot.py (7 GiB, so --jobs 3), inside a private mount namespace
# where the snapshot is bound at private/cfr/snap and its output root at
# private/cfr/out (AUDIT-CANONICAL-PATHS pin_run.sh, OLD=no), through the
# AUDIT-CANONICAL-PATHS driver measure.py (--inputs original). Both sides thus
# spell analyzer, profiles, contracts, inputs and outputs identically.
# SOURCE_DATE_EPOCH is fixed on both sides so Clang's __DATE__/__TIME__ in
# lib/monkey/mk_bin/monkey.c (MONKEY_BUILT) does not depend on the clock
# (the AUDIT-CANONICAL-PATHS clock-dependent key).
# Each side runs in a systemd user scope without a memory limit (cgmeasure.sh)
# for the peak-RSS / pressure observation.
# Usage: run_routes.sh WORKTREE BASE_REV TIP_REV RUNS_DIR [ROUTES]
set -u
W=$1
BASE_REV=$2
TIP_REV=$3
R=$4
A=$W/private/cfr
L=$W/docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json
DRIVER=$W/docs/reports/audit-canonical-paths/scripts/measure.py
PIN=$W/docs/reports/audit-canonical-paths/scripts/pin_run.sh
CGM=$W/docs/reports/cfg-release/scripts/cgmeasure.sh
SLOT=$W/scripts/gate_slot.py
export SOURCE_DATE_EPOCH=1761004800
mkdir -p "$R" || exit 2
side() {
  name=$1
  route=$2
  case $name in
    base) rev=$BASE_REV ;;
    tip) rev=$TIP_REV ;;
  esac
  mkdir -p "$R/$name" || return 2
  python3 "$SLOT" audit --memory-budget 7G --receipt "$R/$name-$route.slot.json" -- \
    systemd-run --user --scope --quiet -p MemoryAccounting=yes --unit="cfr-$name-$route-$$" -- \
    sh "$CGM" "$R/$name-$route.mem" \
    unshare -rm sh "$PIN" "$A/snap-$name" "$A/snap" "$R/$name" "$A/out" no \
    python3 "$DRIVER" "$route" "$A/out/$route" --inputs original --binary-source "$rev" \
    --link-map "$L" --inputs-snapshot "$A/snap" --jobs 3 >"$R/$name-$route.driver.log" 2>&1
  echo $? >"$R/$name-$route.driver.exit"
}
for route in ${5:-corpus slice system}; do
  date -u +%Y-%m-%dT%H:%M:%SZ >"$R/$route.start"
  for name in base tip; do
    side "$name" "$route" &
  done
  wait
done
date -u +%Y-%m-%dT%H:%M:%SZ >"$R/done"
