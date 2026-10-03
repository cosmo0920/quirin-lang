#!/usr/bin/env bash
# profile-run.sh NAME BINARY [ROUTES] [INTERVAL] -- GATEWAY-ANALYSIS-PATH-OPTIMIZATION,
# measurement only: the corpus arbiter's own command and the system route under
# scripts/gdb_sampler.py (wall-clock stack samples every INTERVAL seconds,
# default 0.5) into private/gapo/NAME[-system]-samples.jsonl. Sampling slows
# the run; its outputs are not identity evidence. Under the shared heavy lock;
# exits to files; nothing piped.
set -u
name="$1"; binary="$2"; routes="${3:-corpus,system}"; interval="${4:-0.5}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/gapo"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
inputs=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19
cd "$repo" || exit 2
case ",$routes," in *,corpus,*)
  mkdir "$out/$name" || exit 2
  GAPO_SAMPLES="$out/$name-samples.jsonl" GAPO_INTERVAL="$interval" QCORE_CLANG=/usr/bin/clang \
  QCORE_CHUNKIO_PACKAGE="$inputs/provider-packages-wave20/qcore.chunkio" \
    flock "$lock" /usr/bin/env -u QCORE_SOURCE_CONTRACTS gdb -q -batch -nx -x "$here/gdb_sampler.py" \
    --args "$binary" pinned --jobs 4 \
    "$inputs/c-gateway-wave-pin/fluent-bit-clang-source" \
    "$inputs/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
    "$out/$name/monkey-source.json" "$out/$name/resource-diagnostics.json" \
    > "$out/$name/gdb.log" 2>&1
  echo $? > "$out/$name.exit";;
esac
case ",$routes," in *,system,*)
  prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
  mkdir -p "$out/$name-system" || exit 2
  GAPO_SAMPLES="$out/$name-system-samples.jsonl" GAPO_INTERVAL="$interval" \
  QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
    flock "$lock" gdb -q -batch -nx -x "$here/gdb_sampler.py" --args "$binary" current \
    --checkout "$prepared/fluent-bit-clang-source" \
    --profile-directory "$repo/contract/profiles/fluent-bit" \
    --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
    --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
    --jobs 4 --report "$out/$name-system/resource-diagnostics.json" \
    > "$out/$name-system/gdb.log" 2>&1
  echo $? > "$out/$name-system.exit";;
esac
