#!/usr/bin/env bash
# before.sh -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (adapted from LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED, from OP-CALLBACK-MEMBER-PATH-RELEASE): build the BEFORE binary from
# the base tree (private/lppr/bin/before) and measure it on every route with
# the null property. Each step's exit is written to a file; nothing is piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$repo/private/lppr"
"$here/build.sh" before > "$repo/private/lppr/before-build.out" 2>&1
echo $? > "$repo/private/lppr/before-build.step.exit"
[ "$(cat "$repo/private/lppr/before-build.step.exit")" = 0 ] || exit 1
NULL=1 "$here/measure-binary.sh" before "$repo/private/lppr/bin/before" > "$repo/private/lppr/before-measure.out" 2>&1
echo $? > "$repo/private/lppr/before-measure.step.exit"
