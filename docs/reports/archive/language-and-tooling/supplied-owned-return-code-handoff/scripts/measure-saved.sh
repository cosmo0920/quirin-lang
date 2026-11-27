#!/usr/bin/env bash
# measure-saved.sh NAME BINARY -- SUPPLIED-OWNED-RETURN-CODE-HANDOFF: measure a
# saved binary (private/sorch/bin/...) on the corpus, system and slice routes
# with the null property, as run NAME (measure-binary.sh); console and exit
# to files.
set -u
name="$1"; binary="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/sorch"
NULL=1 "$here/measure-binary.sh" "$name" "$binary" > "$out/measure-$name.console" 2>&1
echo $? > "$out/measure-$name.console.exit"
