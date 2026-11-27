#!/usr/bin/env bash
# before.sh -- HANDOFF-CANDIDATE-OWN-CALLBACKS (adapted from OP-CALLBACK-MEMBER-PATH-RELEASE): build the BEFORE binary from
# the base tree (private/hcoc/bin/before) and measure it on every route with
# the null property. Each step's exit is written to a file; nothing is piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$repo/private/hcoc"
"$here/build.sh" before > "$repo/private/hcoc/before-build.out" 2>&1
echo $? > "$repo/private/hcoc/before-build.step.exit"
[ "$(cat "$repo/private/hcoc/before-build.step.exit")" = 0 ] || exit 1
NULL=1 "$here/measure-binary.sh" before "$repo/private/hcoc/bin/before" > "$repo/private/hcoc/before-measure.out" 2>&1
echo $? > "$repo/private/hcoc/before-measure.step.exit"
