#!/bin/sh
# Re-run the base-namespace corpus route with the sibling-preserving pin_run.sh
# (attempt 2's namespace corpus hid /home/cosmo/Documents/GitHub/monkey, the
# libevent provider lock's tip repository). Same driver, snapshot, mounts and
# slot as run_pinned_paths.sh; output under private/acp/runs3/.
# Usage: rerun_namespace_corpus.sh WORKTREE BASE_REV
set -u
W=$1
A=$W/private/acp
R=$A/runs3
mkdir -p "$R/base-namespace" || exit 2
python3 "$W/scripts/gate_slot.py" audit --memory-budget 7G --receipt "$R/base-namespace-corpus.slot.json" -- \
  unshare -rm sh "$W/docs/reports/audit-canonical-paths/scripts/pin_run.sh" "$A/snap-base-namespace" "$A/snap" \
  "$R/base-namespace" "$A/out" yes \
  python3 "$W/docs/reports/audit-canonical-paths/scripts/measure.py" corpus "$A/out/corpus" --inputs namespace \
  --binary-source "$2" --link-map "$W/docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json" \
  --inputs-snapshot "$A/snap" --jobs 3 >"$R/base-namespace-corpus.driver.log" 2>&1
echo $? >"$R/base-namespace-corpus.driver.exit"
