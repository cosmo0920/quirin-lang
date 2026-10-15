#!/usr/bin/env bash
# after.sh NAME BINARY -- HANDOFF-CANDIDATE-OWN-CALLBACKS (adapted from OP-CALLBACK-MEMBER-PATH-RELEASE): measure a saved
# binary (built by build.sh from the card's tree, whose contracts it reads at
# run time) on every route with the null property, under the shared heavy
# lock (measure-binary.sh). The step's exit goes to private/hcoc/NAME.step.exit.
set -u
name="$1"; binary="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
NULL=1 "$here/measure-binary.sh" "$name" "$binary" > "$repo/private/hcoc/$name-measure.out" 2>&1
echo $? > "$repo/private/hcoc/$name.step.exit"
