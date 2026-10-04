#!/usr/bin/env bash
# runs.sh SIDE -- GAPO-PARITY-DECISIONS-NONDETERMINISTIC: five corpus runs
# and two system and slice runs of private/gapo-nd/bin/SIDE (SIDE1..SIDE5;
# the first two also run the system route and the slice). Exits to files.
set -u
side="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
for n in 1 2 3 4 5; do
  routes=corpus
  [ "$n" -le 2 ] && routes=corpus,system,slice
  "$here/measure.sh" "$side$n" "$repo/private/gapo-nd/bin/$side" "$routes"
done
echo done
