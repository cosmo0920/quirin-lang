#!/bin/sh
# AUDIT-CANONICAL-PATHS identity run: five sides x three routes, run under
# `flock /tmp/qcore-c-card-validation-20261022.lock`.
#   base-mirror     BASE through the re-spelled mirror, by the scheduler card's
#                   own driver (docs/reports/w48-bu-single-leaf-bodies/scripts)
#   tip-mirror      TIP, the same driver and mirror
#   tip-original    TIP on the ORIGINAL unmirrored inputs (measure.py --inputs original)
#   base-namespace  BASE on the original inputs, where the old spelling is a
#                   real directory (unshare -rm ns_run.sh; measure.py --inputs namespace)
#   base-original   BASE on the original inputs as they are (the run-4 failure,
#                   reproduced; expected to exit nonzero)
# Usage: run_identity.sh WORKTREE BASE_REV TIP_REV
set -u
W=$1
BASE_REV=$2
TIP_REV=$3
A=$W/private/acp
R=$A/runs
L=$W/docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json
MIRROR_DRIVER=$W/docs/reports/w48-bu-single-leaf-bodies/scripts/measure.py
DRIVER=$W/docs/reports/audit-canonical-paths/scripts/measure.py
NS=$W/docs/reports/audit-canonical-paths/scripts/ns_run.sh
mkdir -p "$R" || exit 2
side() {
  name=$1
  route=$2
  case $name in
    base-mirror) python3 "$MIRROR_DRIVER" "$route" "$R/$name/$route" --link-map "$L" \
        --inputs-snapshot "$A/base-src" --binary-source "$BASE_REV" --jobs 4 --inputs-mirror "$A/mirror" ;;
    tip-mirror) python3 "$MIRROR_DRIVER" "$route" "$R/$name/$route" --link-map "$L" \
        --inputs-snapshot "$A/tip-src" --binary-source "$TIP_REV" --jobs 4 --inputs-mirror "$A/mirror" ;;
    tip-original) python3 "$DRIVER" "$route" "$R/$name/$route" --link-map "$L" \
        --inputs-snapshot "$A/tip-src" --binary-source "$TIP_REV" --jobs 4 --inputs original ;;
    base-original) python3 "$DRIVER" "$route" "$R/$name/$route" --link-map "$L" \
        --inputs-snapshot "$A/base-src" --binary-source "$BASE_REV" --jobs 4 --inputs original ;;
    base-namespace) unshare -rm sh "$NS" python3 "$DRIVER" "$route" "$R/$name/$route" --link-map "$L" \
        --inputs-snapshot "$A/base-src" --binary-source "$BASE_REV" --jobs 4 --inputs namespace ;;
  esac >"$R/$name-$route.driver.log" 2>&1
  echo $? >"$R/$name-$route.driver.exit"
}
# Three phases bound memory (a corpus audit holds about 7 GB): every side's
# corpus, then every slice, then every system route.
for route in corpus slice system; do
  date -u +%Y-%m-%dT%H:%M:%SZ >"$R/$route.start"
  for name in base-mirror tip-mirror tip-original base-namespace base-original; do
    side "$name" "$route" &
  done
  wait
done
date -u +%Y-%m-%dT%H:%M:%SZ >"$R/done"
