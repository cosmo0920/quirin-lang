#!/usr/bin/env bash
# before.sh -- ROUTE-SELECTION-ORDER-RESTATED: the BEFORE measurements on the
# checked-out base, in order: the release binary (private/rsor/bin/before),
# the tracked route census and the synthetic selector census. Each step under
# the shared heavy lock (inside the scripts it calls); exits to files.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
bash "$here/build.sh" before
bash "$here/census.sh" before
bash "$here/selector-census.sh" before
