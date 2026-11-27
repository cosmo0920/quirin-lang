#!/usr/bin/env bash
# profile-run.sh NAME BINARY [INTERVAL] -- DEEP-AST-TRANSPORTS-AND-WALL-TIME
# (adapted from GATEWAY-ANALYSIS-PATH-OPTIMIZATION's scripts/profile-run.sh),
# measurement only: the corpus arbiter's own command (run_pinned_audit.py's
# arguments and environment) run by BINARY -- a `unit-timing` build -- under
# scripts/gdb_sampler.py (wall-clock stack samples every INTERVAL seconds,
# default 0.5) into private/dat/NAME-samples.jsonl, with the unit intervals in
# private/dat/NAME-units.jsonl. Sampling slows the run; its outputs are not
# identity evidence. A second variant without gdb (NAME ending in -plain) only
# records the unit intervals, for the units' own wall times. Under the shared
# heavy lock; exits to files; nothing piped.
set -u
name="$1"; binary="$2"; interval="${3:-0.5}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/dat"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
inputs=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19
cd "$repo" || exit 2
mkdir "$out/$name" || exit 2
sha256sum "$binary" > "$out/$name.binary.sha256"
case "$name" in
  *-plain)
    QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" QCORE_UNIT_TIMING="$out/$name-units.jsonl" QCORE_CLANG=/usr/bin/clang \
    QCORE_CHUNKIO_PACKAGE="$inputs/provider-packages-wave20/qcore.chunkio" \
      flock "$lock" /usr/bin/env -u QCORE_SOURCE_CONTRACTS bash -c '
        seconds="$1"; shift
        start=$EPOCHREALTIME
        "$@"
        rc=$?
        end=$EPOCHREALTIME
        awk -v s="$start" -v e="$end" "BEGIN { printf \"%.3f\n\", e - s }" > "$seconds"
        exit $rc' _ "$out/$name.seconds" "$binary" pinned --jobs 4 \
      "$inputs/c-gateway-wave-pin/fluent-bit-clang-source" \
      "$inputs/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
      "$out/$name/monkey-source.json" "$out/$name/resource-diagnostics.json" \
      > "$out/$name/console.log" 2> "$out/$name/stderr.log"
    ;;
  *)
    GAPO_SAMPLES="$out/$name-samples.jsonl" GAPO_INTERVAL="$interval" \
    QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" QCORE_UNIT_TIMING="$out/$name-units.jsonl" QCORE_CLANG=/usr/bin/clang \
    QCORE_CHUNKIO_PACKAGE="$inputs/provider-packages-wave20/qcore.chunkio" \
      flock "$lock" /usr/bin/env -u QCORE_SOURCE_CONTRACTS gdb -q -batch -nx -x "$here/gdb_sampler.py" \
      --args "$binary" pinned --jobs 4 \
      "$inputs/c-gateway-wave-pin/fluent-bit-clang-source" \
      "$inputs/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
      "$out/$name/monkey-source.json" "$out/$name/resource-diagnostics.json" \
      > "$out/$name/gdb.log" 2>&1
    ;;
esac
echo $? > "$out/$name.exit"
