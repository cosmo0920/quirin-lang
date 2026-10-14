#!/usr/bin/env bash
# stress.sh RUNS -- PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS: the
# recorded campaign. Release test binaries saved privately as
# private/ppb/bin/{base-rel,tip-rel}/sdk (base: the test-only commit; tip: the
# fix), each run RUNS times at two amplifier settings, base and tip
# alternating per setting. Outputs under private/ppb/stress/.
set -u
runs="${1:-50}"
here="$(cd "$(dirname "$0")" && pwd)"
for setting in "100000 200" "20000 20"; do
  set -- $setting
  for label in base-rel tip-rel; do
    CLOSE_US=$1 CHILD_MS=$2 TAG="$label-close$1-stall$2" \
      bash "$here/campaign.sh" "$label" "$runs"
  done
done
