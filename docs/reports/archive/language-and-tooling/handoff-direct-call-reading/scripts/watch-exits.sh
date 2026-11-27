#!/usr/bin/env bash
# watch-exits.sh STOP-FILE NAME... -- HANDOFF-DIRECT-CALL-READING: print each
# private/hdcr/NAME exit file once it appears (one line per file), and exit
# when STOP-FILE appears. A waiting aid, not evidence.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
out="$(cd "$here/../../../.." && pwd)/private/hdcr"
stop="$1"; shift
declare -A seen
while true; do
  for f in "$@"; do
    if [ -f "$out/$f" ] && [ -z "${seen[$f]:-}" ]; then
      echo "$f = $(cat "$out/$f")"
      seen[$f]=1
    fi
  done
  [ -f "$out/$stop" ] && exit 0
  sleep 20
done
