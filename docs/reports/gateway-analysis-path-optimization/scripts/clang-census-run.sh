#!/usr/bin/env bash
# clang-census-run.sh NAME BINARY [ROUTES] -- GATEWAY-ANALYSIS-PATH-OPTIMIZATION,
# measurement only: the corpus arbiter's own command (run.json's, run directly
# rather than through run_pinned_audit.py, which pins QCORE_CLANG) and the system
# route, with QCORE_CLANG set to scripts/clang-census.sh so every Clang launch is
# logged to private/gapo/NAME-clang.jsonl (NAME-system-clang.jsonl). The real
# Clang is the route's own (/usr/bin/clang for the corpus, llvm-21 for the
# system route; the same executable). Outputs are not identity evidence (the
# wrapper is the recorded Clang program). Under the shared heavy lock; exits to
# files; nothing piped.
set -u
name="$1"; binary="$2"; routes="${3:-corpus,system}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/gapo"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
inputs=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19
mkdir -p "$out/$name-census-tmp" || exit 2
cd "$repo" || exit 2
export QCORE_CLANG_CENSUS_TMP="$out/$name-census-tmp"
case ",$routes," in *,corpus,*)
  mkdir "$out/$name" || exit 2
  QCORE_CLANG="$here/clang-census.sh" QCORE_CLANG_REAL=/usr/bin/clang \
  QCORE_CLANG_CENSUS_LOG="$out/$name-clang.jsonl" \
  QCORE_CHUNKIO_PACKAGE="$inputs/provider-packages-wave20/qcore.chunkio" \
    flock "$lock" /usr/bin/env -u QCORE_SOURCE_CONTRACTS "$binary" pinned --jobs 4 \
    "$inputs/c-gateway-wave-pin/fluent-bit-clang-source" \
    "$inputs/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
    "$out/$name/monkey-source.json" "$out/$name/resource-diagnostics.json" \
    > "$out/$name/audit.log" 2>&1
  echo $? > "$out/$name.exit";;
esac
case ",$routes," in *,system,*)
  prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
  mkdir -p "$out/$name-system" || exit 2
  QCORE_CLANG="$here/clang-census.sh" QCORE_CLANG_REAL=/usr/lib/llvm-21/bin/clang \
  QCORE_CLANG_CENSUS_LOG="$out/$name-system-clang.jsonl" \
    flock "$lock" "$binary" current \
    --checkout "$prepared/fluent-bit-clang-source" \
    --profile-directory "$repo/contract/profiles/fluent-bit" \
    --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
    --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
    --jobs 4 --report "$out/$name-system/resource-diagnostics.json" \
    > "$out/$name-system/console.log" 2> "$out/$name-system/stderr.log"
  echo $? > "$out/$name-system.exit";;
esac
