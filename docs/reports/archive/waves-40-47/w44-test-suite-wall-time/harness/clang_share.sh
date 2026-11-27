#!/bin/bash
# Usage (from the worktree root): clang_share.sh <outdir> <cargo test args...>
# Runs one already-built test binary through cargo with every Clang child
# process timed by clang_timing_wrapper.sh, and reports wall time against the
# summed (possibly overlapping) Clang time.
out="$1"
shift
here="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$out"
export CARGO_INCREMENTAL=0
export CLANG_TIMING_LOG="$(realpath "$out")/clang.log"
export QCORE_CLANG="$here/clang_timing_wrapper.sh"
: >"$CLANG_TIMING_LOG"
uptime >"$out/uptime"
/usr/bin/time -v -o "$out/time.txt" cargo test "$@" >"$out/out.log" 2>&1
echo $? >"$out/exit"
uptime >>"$out/uptime"
{
  grep -E "Elapsed|User time|System time" "$out/time.txt"
  echo "clang invocations: $(wc -l <"$CLANG_TIMING_LOG")"
  echo "clang seconds (summed): $(awk '{s+=$1} END {print s}' "$CLANG_TIMING_LOG")"
} | tee "$out/report.txt"
