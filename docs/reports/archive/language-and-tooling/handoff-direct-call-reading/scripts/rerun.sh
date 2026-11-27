#!/usr/bin/env bash
# rerun.sh NAME BINARY ROUTES -- HANDOFF-DIRECT-CALL-READING: measure a saved
# binary on the given routes (comma list) with the null property, as after.sh
# does for all three; used for the stability reruns. Exit to
# private/hdcr/NAME.step.exit.
set -u
name="$1"; binary="$2"; routes="$3"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
NULL=1 "$here/measure-binary.sh" "$name" "$binary" "$routes" > "$repo/private/hdcr/$name-measure.out" 2>&1
echo $? > "$repo/private/hdcr/$name.step.exit"
