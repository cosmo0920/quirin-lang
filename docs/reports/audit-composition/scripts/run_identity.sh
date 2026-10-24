#!/bin/sh
# AUDIT-COMPOSITION identity run: BASE and TIP over the pinned eight-home
# corpus, the system selection and the tail-pause slice, through the
# W48-BU-SINGLE-LEAF-BODIES driver and its re-spelled inputs mirror
# (docs/reports/w48-bu-single-leaf-bodies/scripts/{measure,mirror_inputs}.py).
# Each route takes an `audit` slot (scripts/gate_slot.py, 7 GiB budget, so at
# most 3 workers: --jobs 3). BASE and TIP of one route run side by side.
# Usage: run_identity.sh WORKTREE RUN_DIR
set -u
W=$1
R=$2
A=$W/private/ac
L=$W/docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json
DRIVER=$W/docs/reports/w48-bu-single-leaf-bodies/scripts/measure.py
mkdir -p "$R" || exit 2
side() {
  name=$1
  route=$2
  python3 "$W/scripts/gate_slot.py" audit --receipt "$R/$name-$route.slot.json" -- \
    python3 "$DRIVER" "$route" "$R/$name/$route" --link-map "$L" \
    --inputs-snapshot "$A/$name-src" --binary-source "$(cat "$A/$name-rev")" --jobs 3 \
    --inputs-mirror "$A/mirror" >"$R/$name-$route.driver.log" 2>&1
  echo $? >"$R/$name-$route.driver.exit"
}
for route in corpus slice system; do
  date -u +%Y-%m-%dT%H:%M:%SZ >"$R/$route.start"
  side base "$route" &
  side tip "$route" &
  wait
  date -u +%Y-%m-%dT%H:%M:%SZ >"$R/$route.end"
done
date -u +%Y-%m-%dT%H:%M:%SZ >"$R/done"
