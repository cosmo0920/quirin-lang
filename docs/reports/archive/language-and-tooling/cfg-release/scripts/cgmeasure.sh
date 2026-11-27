#!/bin/sh
# CFG-RELEASE memory observation of one route. Run as the command of a
# `systemd-run --user --scope -p MemoryAccounting=yes` scope (no memory limit),
# so this shell and everything it starts are the scope's only processes.
# Runs CMD under /usr/bin/time -v (its "Maximum resident set size" is the
# largest single waited-for descendant, the quirin-audit process), then records
# the scope cgroup's memory.peak, memory.swap.peak, memory.events,
# memory.pressure and memory.stat before the scope ends.
# Usage: cgmeasure.sh OUT_PREFIX CMD [ARG...]  -> OUT_PREFIX.time, OUT_PREFIX.cgroup
set -u
out=$1
shift
cg=$(sed -n 's/^0:://p' /proc/self/cgroup)
dir=/sys/fs/cgroup$cg
/usr/bin/time -v -o "$out.time" "$@"
rc=$?
{
  echo "exit $rc"
  echo "cgroup $cg"
  for file in memory.peak memory.swap.peak memory.max memory.events memory.pressure memory.stat; do
    echo "== $file"
    cat "$dir/$file" 2>&1
  done
} >"$out.cgroup"
exit $rc
