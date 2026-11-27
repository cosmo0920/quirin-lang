#!/usr/bin/env bash
# profile-system.sh NAME BINARY [INTERVAL] -- DEEP-AST-TRANSPORTS-AND-WALL-TIME
# (the system branch of GATEWAY-ANALYSIS-PATH-OPTIMIZATION's profile-run.sh),
# measurement only: the system route (measure.sh's command, parity stream on)
# under scripts/gdb_sampler.py, samples into private/dat/NAME-samples.jsonl.
# Sampling slows the run; its outputs are not identity evidence. Under the
# shared heavy lock; exits to files; nothing piped.
set -u
name="$1"; binary="$2"; interval="${3:-0.2}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/dat"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
cd "$repo" || exit 2
mkdir "$out/$name" || exit 2
sha256sum "$binary" > "$out/$name.binary.sha256"
GAPO_SAMPLES="$out/$name-samples.jsonl" GAPO_INTERVAL="$interval" \
QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
  flock "$lock" gdb -q -batch -nx -x "$here/gdb_sampler.py" --args "$binary" current \
  --checkout "$prepared/fluent-bit-clang-source" \
  --profile-directory "$repo/contract/profiles/fluent-bit" \
  --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
  --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
  --jobs 4 --report "$out/$name/resource-diagnostics.json" \
  > "$out/$name/gdb.log" 2>&1
echo $? > "$out/$name.exit"
