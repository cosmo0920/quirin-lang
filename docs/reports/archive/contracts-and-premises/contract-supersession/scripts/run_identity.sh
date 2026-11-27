#!/bin/sh
# CONTRACT-SUPERSESSION identity run: BASE and TIP over the pinned eight-home
# corpus, the tail-pause slice and the system selection, through the
# W48-BU-SINGLE-LEAF-BODIES driver and its re-spelled inputs mirror, exactly as
# AUDIT-COMPOSITION ran them, with one change: SOURCE_DATE_EPOCH is set to one
# value on both sides, so Monkey's mk_bin/monkey.c `__DATE__`/`__TIME__`
# expand identically and the lifetime-source pre-pass key no longer varies.
# Each side runs in its own mount namespace with its snapshot at the common
# path private/cs/side-src; each route takes an `audit` slot (7 GiB, --jobs 3);
# BASE and TIP of one route run side by side.
# Usage: run_identity.sh WORKTREE RUN_DIR LOG_DIR [ROUTE...]
set -u
W=$1
R=$2
LOGS=$3
shift 3
ROUTES=${*:-corpus slice system}
A=$W/private/cs
S=$W/docs/reports/contract-supersession/scripts
L=$W/docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json
DRIVER=$W/docs/reports/w48-bu-single-leaf-bodies/scripts/measure.py
export SOURCE_DATE_EPOCH=1790357241
mkdir -p "$R" "$LOGS" || exit 2
side() {
  name=$1
  route=$2
  unshare -rm sh "$S/ns_side.sh" "$W" "$name" \
    python3 "$W/scripts/gate_slot.py" audit --receipt "$LOGS/$name-$route.slot.json" -- \
    python3 "$DRIVER" "$route" "$R/$name/$route" --link-map "$L" \
    --inputs-snapshot "$A/side-src" --binary-source "$(cat "$A/$name-rev")" --jobs 3 \
    --inputs-mirror "$A/mirror" >"$LOGS/$name-$route.driver.log" 2>&1
  echo $? >"$LOGS/$name-$route.driver.exit"
}
for route in $ROUTES; do
  date -u +%Y-%m-%dT%H:%M:%SZ >"$LOGS/$route.start"
  side base "$route" &
  side tip "$route" &
  wait
  date -u +%Y-%m-%dT%H:%M:%SZ >"$LOGS/$route.end"
done
date -u +%Y-%m-%dT%H:%M:%SZ >"$LOGS/done"
