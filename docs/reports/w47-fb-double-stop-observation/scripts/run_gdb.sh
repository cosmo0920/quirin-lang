#!/usr/bin/env bash
# run_gdb.sh TREE RUN_DIR -- one gdb run of each scenario (debug variant,
# no interposer) under the watchdog, with gdb_joins.gdb; RUN_DIR/gdb-S.log
# and .exit. gdb's own exit is recorded; the inferior's outcome is in the log.
set -u
tree="$1"; run="$2"
here="$(cd "$(dirname "$0")" && pwd)"
dir=/home/cosmo/Documents/GitHub/qcore-c/private/w47-dstop/attempt-1/harness-$tree-debug
mkdir -p "$run" || exit 2
cd "$dir" || exit 2
ulimit -c 0
for s in control v i iii; do
  echo "timeout -k 5 120 gdb -q -batch -nx -x gdb_joins.gdb --args ./harness $s" > "$run/gdb-$s.cmd"
  timeout -k 5 120 gdb -q -batch -nx -x "$here/gdb_joins.gdb" --args ./harness "$s" > "$run/gdb-$s.log" 2>&1
  echo $? > "$run/gdb-$s.exit"
done
for s in control v i iii; do printf '%s=%s ' "$s" "$(cat "$run/gdb-$s.exit")"; done; echo
