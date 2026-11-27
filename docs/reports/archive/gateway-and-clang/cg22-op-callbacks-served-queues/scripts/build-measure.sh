#!/usr/bin/env bash
# build-measure.sh NAME -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES: build the
# release binary NAME from this branch and measure it on every route with the
# null property (scripts/build.sh, scripts/measure-binary.sh, NULL=1).
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
"$here/build.sh" "$name" || exit 1
repo="$(cd "$here/../../../.." && pwd)"
NULL=1 "$here/measure-binary.sh" "$name" "$repo/private/cg22op/bin/$name"
echo "measured $name"
