#!/usr/bin/env bash
# before.sh [NAME] -- RELATIONAL-REWRITE-LINE-ORDER (adapted from HANDOFF-DIRECT-CALL-READING):
# build the checked-out tree's release binary as private/rrlo/bin/NAME (default
# `census`: the base code plus the census rows, which go only to the parity
# stream) and measure it on every route with the null property. Each step's
# exit is written to a file; nothing is piped.
set -u
name="${1:-census}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$repo/private/rrlo"
"$here/build.sh" "$name" > "$repo/private/rrlo/$name-build.out" 2>&1
echo $? > "$repo/private/rrlo/$name-build.step.exit"
[ "$(cat "$repo/private/rrlo/$name-build.step.exit")" = 0 ] || exit 1
NULL=1 "$here/measure-binary.sh" "$name" "$repo/private/rrlo/bin/$name" > "$repo/private/rrlo/$name-measure.out" 2>&1
echo $? > "$repo/private/rrlo/$name-measure.step.exit"
