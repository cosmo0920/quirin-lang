#!/usr/bin/env bash
# unit-series.sh BEFORE-TIMING AFTER-TIMING NAME:SIDE... -- DEEP-AST-TRANSPORTS-AND-WALL-TIME:
# alternating unsampled `unit-timing` corpus runs (profile-run.sh NAME-plain),
# for every unit's own analysis seconds on both sides under the same host
# conditions; waits for WAIT (a file) when set.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
before="$1"; after="$2"; shift 2
if [ -n "${WAIT:-}" ]; then
  until [ -f "$WAIT" ]; do sleep 15; done
fi
for item in "$@"; do
  name="${item%%:*}"; side="${item##*:}"
  case "$side" in
    before) "$here/profile-run.sh" "$name-plain" "$before";;
    after) "$here/profile-run.sh" "$name-plain" "$after";;
    *) echo "unknown side $side"; exit 2;;
  esac
done
