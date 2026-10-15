#!/usr/bin/env bash
# before.sh -- OP-CALLBACK-MEMBER-PATH-RELEASE: build the BEFORE binary from
# the base tree (private/mpr/bin/before) and measure it on every route with
# the null property. Each step's exit is written to a file; nothing is piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$repo/private/mpr"
"$here/build.sh" before > "$repo/private/mpr/before-build.out" 2>&1
echo $? > "$repo/private/mpr/before-build.step.exit"
[ "$(cat "$repo/private/mpr/before-build.step.exit")" = 0 ] || exit 1
NULL=1 "$here/measure-binary.sh" before "$repo/private/mpr/bin/before" > "$repo/private/mpr/before-measure.out" 2>&1
echo $? > "$repo/private/mpr/before-measure.step.exit"
