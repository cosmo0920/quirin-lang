#!/bin/sh
# AUDIT-CANONICAL-PATHS identity run (attempt 2). Each route of each side takes
# one `audit` slot of scripts/gate_slot.py (7 GiB budget, so --jobs 3, the
# budget's worker cap; every artifact is byte-identical at any job count,
# docs/agents/validation.md). Each side has its own
# git-archive snapshot (with the pinned libevent submodule), built and run in
# a private mount namespace where it is bound at private/acp/snap and its
# output root at private/acp/out (pin_run.sh), so all sides share one
# spelling of analyzer, profiles, contracts and output, and differ only in
# how the inputs are spelled:
#   base-mirror     BASE through the re-spelled mirror, by the scheduler card's
#                   own driver (docs/reports/w48-bu-single-leaf-bodies/scripts)
#   tip-mirror      TIP, the same driver and mirror
#   tip-original    TIP on the ORIGINAL unmirrored inputs (measure.py --inputs original)
#   base-namespace  BASE on the original inputs where the old spelling is a
#                   real directory (pin_run.sh OLD=yes; measure.py --inputs namespace)
# Usage: run_pinned_paths.sh WORKTREE BASE_REV TIP_REV RUNS_DIR_NAME [ROUTES]
set -u
W=$1
BASE_REV=$2
TIP_REV=$3
A=$W/private/acp
R=$A/$4
L=$W/docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json
MIRROR_DRIVER=$W/docs/reports/w48-bu-single-leaf-bodies/scripts/measure.py
DRIVER=$W/docs/reports/audit-canonical-paths/scripts/measure.py
PIN=$W/docs/reports/audit-canonical-paths/scripts/pin_run.sh
SLOT=$W/scripts/gate_slot.py
mkdir -p "$R" || exit 2
side() {
  name=$1
  route=$2
  mkdir -p "$R/$name" || return 2
  case $name in
    base-mirror) set -- base-mirror no python3 "$MIRROR_DRIVER" "$route" "$A/out/$route" --inputs-mirror "$A/mirror" --binary-source "$BASE_REV" ;;
    tip-mirror) set -- tip-mirror no python3 "$MIRROR_DRIVER" "$route" "$A/out/$route" --inputs-mirror "$A/mirror" --binary-source "$TIP_REV" ;;
    tip-original) set -- tip-original no python3 "$DRIVER" "$route" "$A/out/$route" --inputs original --binary-source "$TIP_REV" ;;
    base-namespace) set -- base-namespace yes python3 "$DRIVER" "$route" "$A/out/$route" --inputs namespace --binary-source "$BASE_REV" ;;
  esac
  snapshot=$A/snap-$1
  old=$2
  shift 2
  python3 "$SLOT" audit --memory-budget 7G --receipt "$R/$name-$route.slot.json" -- \
    unshare -rm sh "$PIN" "$snapshot" "$A/snap" "$R/$name" "$A/out" "$old" \
    "$@" --link-map "$L" --inputs-snapshot "$A/snap" --jobs 3 >"$R/$name-$route.driver.log" 2>&1
  echo $? >"$R/$name-$route.driver.exit"
}
# Three phases bound memory (a corpus audit holds about 7 GB).
for route in ${5:-corpus slice system}; do
  date -u +%Y-%m-%dT%H:%M:%SZ >"$R/$route.start"
  for name in base-mirror tip-mirror tip-original base-namespace; do
    side "$name" "$route" &
  done
  wait
done
date -u +%Y-%m-%dT%H:%M:%SZ >"$R/done"
