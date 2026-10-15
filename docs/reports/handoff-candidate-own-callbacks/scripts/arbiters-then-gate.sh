#!/usr/bin/env bash
# arbiters-then-gate.sh MAIN MAINNAME TIPNAME GATENAME -- HANDOFF-CANDIDATE-OWN-CALLBACKS:
# with both binaries already saved (build-main.sh, build.sh), recreate MAIN's
# export (EXPORT_ONLY=1), measure both binaries on every route with the null
# property one after the other (after.sh), then run the full standing gate
# and the provider gate (gate-run.sh GATENAME) only when at least 80 GB are
# free (the workspace test build alone needs about 57 GB; two earlier gate
# attempts failed on a full shared disk). Free space before each step goes to
# private/hcoc/order-*.df.
set -u
main="$1"; mainname="$2"; tipname="$3"; gatename="$4"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/hcoc"
EXPORT_ONLY=1 bash "$here/build-main.sh" "$main" "$mainname" > "$out/$mainname-export.out" 2>&1
echo $? > "$out/order-export-$mainname.exit"
df -h / > "$out/order-$mainname.df"
bash "$here/after.sh" "$mainname" "$out/bin/$mainname"
df -h / > "$out/order-$tipname.df"
bash "$here/after.sh" "$tipname" "$out/bin/$tipname"
until [ "$(df --output=avail -BG / | tail -1 | tr -dc 0-9)" -ge 80 ]; do sleep 60; done
df -h / > "$out/order-$gatename.df"
bash "$here/gate-run.sh" "$gatename" > "$out/gate-run-$gatename.out" 2>&1
echo done > "$out/order.done"
