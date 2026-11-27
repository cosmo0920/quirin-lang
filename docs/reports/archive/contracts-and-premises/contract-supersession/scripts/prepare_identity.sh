#!/bin/sh
# CONTRACT-SUPERSESSION identity run preparation: the re-spelled inputs mirror
# (docs/reports/w48-bu-single-leaf-bodies/scripts/mirror_inputs.py), one
# snapshot per side, and both release builds side by side, each in its own
# mount namespace at the common path.
# Usage: prepare_identity.sh WORKTREE LOG_DIR BASE_REV TIP_REV
set -u
W=$1
LOGS=$2
base=$3
tip=$4
S=$W/docs/reports/contract-supersession/scripts
mkdir -p "$LOGS" "$W/private/cs"
cd "$W" || exit 2
if [ ! -f "$W/private/cs/mirror/manifest.json" ]; then
  python3 docs/reports/w48-bu-single-leaf-bodies/scripts/mirror_inputs.py "$W/private/cs/mirror" \
    >"$LOGS/mirror.log" 2>&1
  echo $? >"$LOGS/mirror.exit"
fi
sh "$S/snapshot.sh" "$W" base "$base" >"$LOGS/snapshot-base.log" 2>&1
echo $? >"$LOGS/snapshot-base.exit"
sh "$S/snapshot.sh" "$W" tip "$tip" >"$LOGS/snapshot-tip.log" 2>&1
echo $? >"$LOGS/snapshot-tip.exit"
for side in base tip; do
  unshare -rm sh "$S/ns_side.sh" "$W" "$side" sh "$S/build_side.sh" "$W" "$LOGS/build-$side" &
done
wait
cat "$LOGS"/*.exit
