#!/usr/bin/env bash
# Generate the filesystem backlog fixture with the unmodified Debug binary:
# a `dummy` input with filesystem storage and two `tcp` outputs to closed
# ports (127.0.0.1:1 and :2) so delivery fails and chunks persist. Fluent Bit
# is stopped by the watchdog (SIGTERM at the cap; exit 124 is expected).
# usage: make-fixture.sh ATTEMPT
set -u
attempt="$1"
here="$(cd "$(dirname "$0")" && pwd)"
report="$(cd "$here/.." && pwd)"
root=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-engine-progress/$attempt
bin="$root/native-debug/bin/fluent-bit"
run="$root/run"
logs="$root/native-logs"
mkdir -p "$run" "$logs"
[ -e "$run/storage" ] && { echo "storage exists; use a fresh attempt"; exit 2; }
timeout --signal=TERM -k 5 12 "$bin" -c "$report/native/conf/phase1-generate.conf" > "$logs/phase1.log" 2>&1 &
pid=$!
echo "$pid" > "$logs/phase1.pid"
wait "$pid"
echo $? > "$logs/phase1.exit"
cp -r "$run/storage" "$run/storage-pristine"
(cd "$run/storage-pristine" && find . -type f -print0 | sort -z | xargs -0 sha256sum) > "$run/storage-pristine.sha256"
(cd "$run/storage-pristine" && find . -type f -printf '%p %s\n' | sort) > "$run/storage-pristine.listing"
echo "phase1 exit=$(cat "$logs/phase1.exit") pid=$pid"
cat "$run/storage-pristine.listing"
