#!/bin/sh
# Recompute every published unit row from its component sidecar
# (crates/qcore-audit/examples/component_check.rs) for each route of a run:
# TIP's report against TIP's sidecar, and BASE's report against TIP's sidecar
# (BASE writes no sidecar; its report is byte-identical to TIP's).
# Usage: check_rederivation.sh WORKTREE RUN_DIR OUT_DIR
set -u
W=$1
R=$2
O=$3
mkdir -p "$O" || exit 2
CHECK=$W/target/debug/examples/component_check
status=0
for route in corpus slice system; do
  dir=$route
  [ "$route" = corpus ] && dir=corpus/corpus
  for side in tip base; do
    "$CHECK" "$R/$side/$dir/resource-diagnostics.json" \
      "$R/tip/$dir/resource-diagnostics.components.json" >"$O/$route-$side.json" 2>"$O/$route-$side.err"
    code=$?
    echo $code >"$O/$route-$side.exit"
    [ $code -eq 0 ] || status=1
  done
done
exit $status
