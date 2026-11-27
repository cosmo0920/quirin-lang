#!/usr/bin/env bash
# Verify every built provider package (constraint 18), one exit file each.
# usage: verify_packages.sh PACKAGE_DIR
set -u
dir="$1"
for name in qcore.chunkio qcore.cmetrics qcore.ctraces qcore.libevent qcore.librdkafka; do
  python3 providers/package.py verify "$dir/$name" > "$dir/verify-$name.log" 2>&1
  echo $? > "$dir/verify-$name.exit"
  echo "$name $(cat "$dir/verify-$name.exit")"
done
