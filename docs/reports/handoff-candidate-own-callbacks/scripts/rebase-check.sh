#!/usr/bin/env bash
# rebase-check.sh MAIN MAINNAME TIPNAME GATENAME -- HANDOFF-CANDIDATE-OWN-CALLBACKS:
# after the rebase onto MAIN, in order (each heavy step takes the shared heavy
# lock itself): build MAIN's binary from an export (build-main.sh MAIN
# MAINNAME) and the rebased tip's (build.sh TIPNAME), run the full standing
# gate and the provider gate (gate-run.sh GATENAME), then measure both
# binaries on every route with the null property, one after the other
# (after.sh). Each step's exit to private/hcoc/rebase-check-*.exit.
set -u
main="$1"; mainname="$2"; tipname="$3"; gatename="$4"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/hcoc"
bash "$here/build-main.sh" "$main" "$mainname" > "$out/$mainname-build.out" 2>&1
echo $? > "$out/rebase-check-build-$mainname.exit"
bash "$here/build.sh" "$tipname" > "$out/$tipname-build.out" 2>&1
echo $? > "$out/rebase-check-build-$tipname.exit"
bash "$here/gate-run.sh" "$gatename" > "$out/gate-run-$gatename.out" 2>&1
echo $? > "$out/rebase-check-gate-$gatename.exit"
bash "$here/after.sh" "$mainname" "$out/bin/$mainname"
bash "$here/after.sh" "$tipname" "$out/bin/$tipname"
echo done > "$out/rebase-check.done"
