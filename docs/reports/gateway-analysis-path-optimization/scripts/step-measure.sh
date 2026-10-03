#!/usr/bin/env bash
# step-measure.sh NAME... -- GATEWAY-ANALYSIS-PATH-OPTIMIZATION: for each NAME,
# measure the saved binary private/gapo/bin/NAME on the three routes
# (measure-binary.sh) and write its identity proof against before1 to
# private/gapo/identity-NAME.json (identity.py; its last line is IDENTICAL or
# DIFFERENT). Exits to files; nothing piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
for name in "$@"; do
  "$here/measure-binary.sh" "$name" "private/gapo/bin/$name"
  python3 "$here/identity.py" before1 "$name" "private/gapo/identity-$name.json" \
    > "private/gapo/identity-$name.txt" 2>&1
  echo $? > "private/gapo/identity-$name.exit"
  tail -1 "private/gapo/identity-$name.txt"
done
