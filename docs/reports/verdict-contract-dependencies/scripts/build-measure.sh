#!/usr/bin/env bash
# build-measure.sh NAME [ROUTES] -- REFCOUNT-RESOURCE-KIND: build.sh NAME, then
# measure-binary.sh NAME private/vcd/bin/NAME ROUTES. Exits to files (the two
# scripts write their own); this script's own exit is the first failing step's.
set -u
name="$1"; routes="${2:-corpus,system,slice}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
bash "$here/build.sh" "$name" || exit 1
bash "$here/measure-binary.sh" "$name" "$repo/private/vcd/bin/$name" "$routes" || exit 2
