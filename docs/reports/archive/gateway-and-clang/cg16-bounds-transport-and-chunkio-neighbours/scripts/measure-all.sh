#!/usr/bin/env bash
# measure-all.sh NAME -- CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: the custody routes
# (corpus arbiter with typed-path parity, system route, tail-pause slice) and
# then the corpus arbiter with the bounds property selected, all with the saved
# binary private/cg16-bounds-transport-and-chunkio-neighbours/bin/NAME. Each step's exit
# is written by the step scripts; this prints one line per step.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"
binary="$repo/private/cg16-bounds-transport-and-chunkio-neighbours/bin/$name"
bash "$here/measure-binary.sh" "$name" "$binary"
echo "custody $name exit $?"
bash "$here/measure-bounds.sh" "$name" "$binary"
echo "bounds $name exit $?"
