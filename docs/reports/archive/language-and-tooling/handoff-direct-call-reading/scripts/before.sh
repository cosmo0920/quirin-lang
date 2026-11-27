#!/usr/bin/env bash
# before.sh -- HANDOFF-DIRECT-CALL-READING (adapted from HANDOFF-CANDIDATE-OWN-CALLBACKS, from OP-CALLBACK-MEMBER-PATH-RELEASE): build the BEFORE binary from
# the base tree (private/hdcr/bin/before) and measure it on every route with
# the null property. Each step's exit is written to a file; nothing is piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$repo/private/hdcr"
"$here/build.sh" before > "$repo/private/hdcr/before-build.out" 2>&1
echo $? > "$repo/private/hdcr/before-build.step.exit"
[ "$(cat "$repo/private/hdcr/before-build.step.exit")" = 0 ] || exit 1
NULL=1 "$here/measure-binary.sh" before "$repo/private/hdcr/bin/before" > "$repo/private/hdcr/before-measure.out" 2>&1
echo $? > "$repo/private/hdcr/before-measure.step.exit"
