#!/bin/sh
# AUDIT-COMPOSITION identity run: BASE and TIP over the pinned eight-home
# corpus, the tail-pause slice and the system selection, through the
# W48-BU-SINGLE-LEAF-BODIES driver and its re-spelled inputs mirror
# (docs/reports/w48-bu-single-leaf-bodies/scripts/{measure,mirror_inputs}.py).
#
# Each side runs in its own mount namespace with its snapshot
# (private/ac/{base,tip}-src, git archive plus third_party/libevent checked out
# at the pinned revision) bound at the common path private/ac/side-src, where
# build_side.sh built it with one explicit QCORE_AUDIT_BUILD_COMMIT. So the
# two sides differ only in their source. Each route takes an `audit` slot
# (scripts/gate_slot.py, 7 GiB budget, so at most 3 workers: --jobs 3); BASE
# and TIP of one route run side by side.
# Usage: run_identity.sh WORKTREE RUN_DIR [ROUTE...]   (default: corpus slice system)
set -u
W=$1
R=$2
shift 2
ROUTES=${*:-corpus slice system}
A=$W/private/ac
S=$W/docs/reports/audit-composition/scripts
L=$W/docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json
DRIVER=$W/docs/reports/w48-bu-single-leaf-bodies/scripts/measure.py
mkdir -p "$R" || exit 2
side() {
  name=$1
  route=$2
  unshare -rm sh "$S/ns_side.sh" "$W" "$name" \
    python3 "$W/scripts/gate_slot.py" audit --receipt "$R/$name-$route.slot.json" -- \
    python3 "$DRIVER" "$route" "$R/$name/$route" --link-map "$L" \
    --inputs-snapshot "$A/side-src" --binary-source "$(cat "$A/$name-rev")" --jobs 3 \
    --inputs-mirror "$A/mirror" >"$R/$name-$route.driver.log" 2>&1
  echo $? >"$R/$name-$route.driver.exit"
}
for route in $ROUTES; do
  date -u +%Y-%m-%dT%H:%M:%SZ >"$R/$route.start"
  side base "$route" &
  side tip "$route" &
  wait
  date -u +%Y-%m-%dT%H:%M:%SZ >"$R/$route.end"
done
date -u +%Y-%m-%dT%H:%M:%SZ >"$R/done"
