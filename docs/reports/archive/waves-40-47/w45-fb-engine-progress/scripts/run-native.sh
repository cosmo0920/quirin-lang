#!/usr/bin/env bash
# One native run of the unmodified Debug Fluent Bit binary under gdb, with a
# `timeout` watchdog. Restores the pristine backlog fixture first (plain copy,
# then a SHA-256 check of every restored chunk against the pristine listing).
# usage: run-native.sh ATTEMPT NAME GDB_SCRIPT SECONDS
# Outputs under ATTEMPT/native-logs/: NAME.log (gdb + Fluent Bit output),
# NAME.exit (the watchdog's exit), NAME.pid (the watchdog's PID),
# NAME.wd.txt (the [wd] lines), NAME.fixture.sha256.
set -u
attempt="$1"; name="$2"; script="$3"; seconds="$4"
here="$(cd "$(dirname "$0")" && pwd)"
report="$(cd "$here/.." && pwd)"
root=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-engine-progress/$attempt
bin="$root/native-debug/bin/fluent-bit"
run="$root/run"
logs="$root/native-logs"
[ -e "$logs/$name.log" ] && { echo "run $name exists; choose a fresh name"; exit 2; }
rm -rf "$run/storage"
cp -r "$run/storage-pristine" "$run/storage"
(cd "$run/storage" && find . -type f -print0 | sort -z | xargs -0 sha256sum) > "$logs/$name.fixture.sha256"
cmp -s "$logs/$name.fixture.sha256" "$run/storage-pristine.sha256" || { echo "fixture restore mismatch"; exit 3; }
timeout -k 5 "$seconds" gdb -batch -x "$report/native/gdb/$script" \
  --args "$bin" -c "$report/native/conf/phase2-backlog.conf" > "$logs/$name.log" 2>&1 &
pid=$!
echo "$pid" > "$logs/$name.pid"
wait "$pid"
echo $? > "$logs/$name.exit"
grep -a '^\[wd\]' "$logs/$name.log" > "$logs/$name.wd.txt"
echo "$name exit=$(cat "$logs/$name.exit") pid=$pid"
tail -3 "$logs/$name.wd.txt"
