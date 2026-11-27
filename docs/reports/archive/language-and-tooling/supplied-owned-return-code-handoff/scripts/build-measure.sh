#!/usr/bin/env bash
# build-measure.sh NAME -- SUPPLIED-OWNED-RETURN-CODE-HANDOFF: build the
# checked-out tree's release quirin-audit as private/sorch/bin/NAME
# (build.sh), then measure it on the corpus, system and slice routes with the
# null property (measure-binary.sh). Each step's console and exit go to files.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/sorch"
mkdir -p "$out" || exit 2
"$here/build.sh" "$name" > "$out/build-$name.console" 2>&1
echo $? > "$out/build-$name.console.exit"
[ "$(cat "$out/build-$name.console.exit")" = 0 ] || exit 1
NULL=1 "$here/measure-binary.sh" "$name" "$out/bin/$name" > "$out/measure-$name.console" 2>&1
echo $? > "$out/measure-$name.console.exit"
