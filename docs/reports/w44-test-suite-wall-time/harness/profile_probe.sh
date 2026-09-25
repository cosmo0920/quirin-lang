#!/bin/bash
# Usage (from the worktree root):
#   profile_probe.sh <outdir> <target-dir> <package> <test> [--config ...]...
# Builds one integration-test binary under a profile override in its own target
# directory, then times running it (the build is timed separately). Used to
# decide between opt-level options before measuring the whole suite.
out="$1"; tdir="$2"; pkg="$3"; test="$4"
shift 4
mkdir -p "$out"
export CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR="$tdir"
uptime >"$out/uptime"
/usr/bin/time -v -o "$out/build-time.txt" cargo test -p "$pkg" --test "$test" --no-run "$@" >"$out/build.log" 2>&1
echo "build exit $?" >"$out/exit"
/usr/bin/time -v -o "$out/run-time.txt" cargo test -p "$pkg" --test "$test" "$@" >"$out/run.log" 2>&1
echo "run exit $?" >>"$out/exit"
uptime >>"$out/uptime"
{
  echo "build: $(grep Elapsed "$out/build-time.txt")"
  echo "run:   $(grep Elapsed "$out/run-time.txt")"
  grep "test result" "$out/run.log"
  cat "$out/exit"
} | tee "$out/report.txt"
