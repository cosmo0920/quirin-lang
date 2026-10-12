#!/usr/bin/env bash
# measure-run.sh NAME BINARY -- ROUTE-SELECTION-ORDER-RESTATED: the corpus
# arbiter, system route and tail-pause slice with the saved BINARY and the
# null property selected (measure-binary.sh with NULL=1), output to
# private/rsor/measure-NAME.out and its exit to measure-NAME.exit.
set -u
name="$1"; binary="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$repo/private/rsor" || exit 2
NULL=1 bash "$here/measure-binary.sh" "$name" "$binary" > "$repo/private/rsor/measure-$name.out" 2>&1
echo $? > "$repo/private/rsor/measure-$name.exit"
cat "$repo/private/rsor/measure-$name.exit"
