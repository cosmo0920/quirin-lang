#!/bin/bash
# QCORE_CLANG wrapper for profiling only: runs the real clang and appends
# "<seconds> <first args>" to $CLANG_TIMING_LOG. Used to measure how much of a
# heavy test binary's wall time is spent inside Clang child processes.
start=$(date +%s.%N)
/usr/bin/clang "$@"
code=$?
end=$(date +%s.%N)
echo "$(echo "$end - $start" | bc) $code $*" | cut -c1-200 >>"${CLANG_TIMING_LOG:-/dev/null}"
exit $code
