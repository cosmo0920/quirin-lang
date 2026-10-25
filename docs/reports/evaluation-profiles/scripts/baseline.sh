#!/bin/bash
# EVALUATION-PROFILES: measure the three live profiles once at BASE 710021bd7
# (corpus = eight-home, system = fluent-bit-system, slice = fluent-bit-tail-pause)
# with the AUDIT-CANONICAL-PATHS route driver, unchanged, from a `git archive`
# snapshot of BASE.  The snapshot's third_party/libevent is a symlink to the
# card worktree's checked-out submodule (pinned d82464a2), so the pinned
# route's Cycle 18 source check reads the pinned revision and no git command
# runs outside the card worktree.  Every output and log is under $P, outside
# the worktree.
set -u
P=/media/Data/GitHub/quirin-lang/private/evaluation-profiles
WT=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-aab8b9fd1a9023c58
BASE=710021bd713b90037e803d4a1f87d95eba6476c9
SNAP=$P/snap
RUNS=$P/runs-1
mkdir -p "$RUNS"
exec >>"$P/baseline.log" 2>&1
echo "start $(date -Is)"
if [ ! -d "$SNAP" ]; then
  mkdir -p "$SNAP"
  git -C "$WT" archive "$BASE" | tar -x -C "$SNAP" || { echo "archive failed"; exit 1; }
  rmdir "$SNAP/third_party/libevent" || exit 1
  ln -s "$WT/third_party/libevent" "$SNAP/third_party/libevent" || exit 1
fi
echo "libevent HEAD $(git -C "$WT/third_party/libevent" rev-parse HEAD)"
echo "build $(date -Is)"
( cd "$SNAP" && CARGO_TARGET_DIR="$SNAP/target" CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 \
  cargo build --release -p qcore-audit --bin quirin-audit ) ; echo "build exit $?"
[ -x "$SNAP/target/release/quirin-audit" ] || exit 1
sha256sum "$SNAP/target/release/quirin-audit"
for route in system slice corpus; do
  (
    python3 "$WT/scripts/gate_slot.py" audit --receipt "$RUNS/$route-slot.json" -- \
      python3 "$SNAP/docs/reports/audit-canonical-paths/scripts/measure.py" "$route" "$RUNS/$route" \
        --link-map "$SNAP/docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json" \
        --inputs-snapshot "$SNAP" --binary-source "$BASE" --inputs original --jobs 3
    echo "$?" > "$RUNS/$route.driver-exit"
    echo "$route done $(date -Is) exit $(cat "$RUNS/$route.driver-exit")"
  ) &
  sleep 5
done
wait
echo "end $(date -Is)"
