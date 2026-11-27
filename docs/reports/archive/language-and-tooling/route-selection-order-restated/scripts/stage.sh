#!/usr/bin/env bash
# stage.sh NAME -- ROUTE-SELECTION-ORDER-RESTATED: before.sh for any stage of
# the checked-out tree: the release binary (private/rsor/bin/NAME), the
# tracked route census and the synthetic selector census, then the corpus,
# system and slice arbiters with that binary and the null property
# (measure-run.sh). Each step under the shared heavy lock; exits to files.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
bash "$here/build.sh" "$name" || exit 1
bash "$here/census.sh" "$name"
bash "$here/selector-census.sh" "$name"
bash "$here/measure-run.sh" "$name" "$repo/private/rsor/bin/$name"
