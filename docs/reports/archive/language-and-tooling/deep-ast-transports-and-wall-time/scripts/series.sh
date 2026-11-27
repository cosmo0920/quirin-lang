#!/usr/bin/env bash
# series.sh BEFORE-BINARY AFTER-BINARY NAME:SIDE... -- DEEP-AST-TRANSPORTS-AND-WALL-TIME:
# an interleaved timing series. Each argument NAME:before or NAME:after runs
# measure-binary.sh NAME with that side's binary on all three routes, in the
# order given, after waiting for WAIT (a file) to exist when set. The host's
# speed drifts over hours, so BEFORE and AFTER are compared from runs taken
# alternately.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
before="$1"; after="$2"; shift 2
if [ -n "${WAIT:-}" ]; then
  until [ -f "$WAIT" ]; do sleep 15; done
fi
for item in "$@"; do
  name="${item%%:*}"; side="${item##*:}"
  case "$side" in
    before) "$here/measure-binary.sh" "$name" "$before";;
    after) "$here/measure-binary.sh" "$name" "$after";;
    *) echo "unknown side $side"; exit 2;;
  esac
done
