#!/usr/bin/env bash
# cpu-system.sh NAME BINARY -- DEEP-AST-TRANSPORTS-AND-WALL-TIME, measurement
# only: the system route (measure.sh's command, parity stream on) under
# cpu_poll.py into private/dat/NAME-cpu.jsonl: the arbiter's own CPU ticks per
# thread every 0.25 s (Clang's processes not counted). Under the shared heavy
# lock; exits to files; nothing piped.
set -u
name="$1"; binary="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/dat"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
cd "$repo" || exit 2
mkdir "$out/$name" || exit 2
sha256sum "$binary" > "$out/$name.binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
  flock "$lock" python3 "$here/cpu_poll.py" "$out/$name-cpu.jsonl" 0.25 "$binary" current \
  --checkout "$prepared/fluent-bit-clang-source" \
  --profile-directory "$repo/contract/profiles/fluent-bit" \
  --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
  --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
  --jobs 4 --report "$out/$name/resource-diagnostics.json" \
  > "$out/$name/console.log" 2> "$out/$name/stderr.log"
echo $? > "$out/$name.exit"
