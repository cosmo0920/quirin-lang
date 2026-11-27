#!/usr/bin/env bash
# One native retry run of the unmodified Debug binary under gdb, with a
# `timeout` watchdog, over a fresh storage directory (native/conf/retry.conf:
# one dummy record, filesystem storage, a tcp output to a closed port,
# retry_limit 3, scheduler.base 1 / scheduler.cap 2).
# usage: run-native-retry.sh ATTEMPT NAME GDB_SCRIPT SECONDS
set -u
attempt="$1"; name="$2"; script="$3"; seconds="$4"
here="$(cd "$(dirname "$0")" && pwd)"
report="$(cd "$here/.." && pwd)"
root=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-engine-progress/$attempt
bin="$root/native-debug/bin/fluent-bit"
logs="$root/native-logs"
[ -e "$logs/$name.log" ] && { echo "run $name exists; choose a fresh name"; exit 2; }
rm -rf "$root/run/retry-storage"
timeout -k 5 "$seconds" gdb -batch -x "$report/native/gdb/$script" \
  --args "$bin" -c "$report/native/conf/retry.conf" > "$logs/$name.log" 2>&1 &
pid=$!
echo "$pid" > "$logs/$name.pid"
wait "$pid"
echo $? > "$logs/$name.exit"
grep -a '^\[wd\]' "$logs/$name.log" > "$logs/$name.wd.txt"
echo "$name exit=$(cat "$logs/$name.exit") pid=$pid"
tail -3 "$logs/$name.wd.txt"
