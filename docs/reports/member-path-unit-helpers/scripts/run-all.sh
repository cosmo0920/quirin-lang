#!/usr/bin/env bash
# run-all.sh AFTER-COMMIT -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: build
# BEFORE (export of cd1239503) and AFTER `a1` (export of AFTER-COMMIT) with
# build-export.sh, then measure each on every route with the null property
# (after.sh), each step under the shared heavy lock. Step exits go to
# private/mpuh/*.exit and private/mpuh/NAME.step.exit.
set -u
commit="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/mpuh"
mkdir -p "$out"
bash "$here/build-export.sh" before cd1239503 > "$out/run-build-before.out" 2>&1
echo $? > "$out/run-build-before.exit"
bash "$here/build-export.sh" a1 "$commit" > "$out/run-build-a1.out" 2>&1
echo $? > "$out/run-build-a1.exit"
[ "$(cat "$out/run-build-before.exit")" = 0 ] && bash "$here/after.sh" before "$out/bin/before"
[ "$(cat "$out/run-build-a1.exit")" = 0 ] && bash "$here/after.sh" a1 "$out/bin/a1"
echo done > "$out/run-all.done"
