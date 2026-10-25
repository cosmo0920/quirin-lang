#!/bin/sh
# Build BASE and TIP side by side, each in its own mount namespace at the
# common path. Usage: build_both.sh WORKTREE
set -u
W=$1
S=$W/docs/reports/audit-composition/scripts
for side in base tip; do
  unshare -rm sh "$S/ns_side.sh" "$W" "$side" sh "$S/build_side.sh" "$W" "$W/private/ac/ns-$side-build" &
done
wait
cat "$W/private/ac/ns-base-build.exit" "$W/private/ac/ns-tip-build.exit"
