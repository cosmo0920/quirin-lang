#!/usr/bin/env bash
# cpu-run.sh NAME BINARY -- DEEP-AST-TRANSPORTS-AND-WALL-TIME, measurement only:
# the corpus arbiter's own command run by BINARY (a `unit-timing` build) under
# cpu_poll.py (every thread's CPU ticks every 0.25 s) into
# private/dat/NAME-cpu.jsonl, with the unit intervals in NAME-units.jsonl and
# the parity stream on, as the measured runs. Under the shared heavy lock;
# exits to files; nothing piped.
set -u
name="$1"; binary="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/dat"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
inputs=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19
cd "$repo" || exit 2
mkdir "$out/$name" || exit 2
sha256sum "$binary" > "$out/$name.binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" QCORE_UNIT_TIMING="$out/$name-units.jsonl" QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE="$inputs/provider-packages-wave20/qcore.chunkio" \
  flock "$lock" /usr/bin/env -u QCORE_SOURCE_CONTRACTS python3 "$here/cpu_poll.py" "$out/$name-cpu.jsonl" 0.25 \
  "$binary" pinned --jobs 4 \
  "$inputs/c-gateway-wave-pin/fluent-bit-clang-source" \
  "$inputs/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
  "$out/$name/monkey-source.json" "$out/$name/resource-diagnostics.json" \
  > "$out/$name/console.log" 2> "$out/$name/stderr.log"
echo $? > "$out/$name.exit"
