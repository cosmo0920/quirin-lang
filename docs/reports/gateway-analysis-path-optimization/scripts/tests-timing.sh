#!/usr/bin/env bash
# tests-timing.sh NAME [CLANG-CENSUS] -- GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the
# standing gate's Rust suite (`python3 scripts/run_test_binaries.py`, default
# jobs) over the checked-out tree, under the shared heavy lock, with its wall
# time measured inside the lock into private/gapo/tests-NAME.seconds and its
# log directory private/gapo/tests-NAME/ (summary.json carries the build,
# capture and run phase seconds). With CLANG-CENSUS=1, QCORE_CLANG is the
# clang-census.sh wrapper (a test that sets QCORE_CLANG itself overrides it),
# logging every launch to private/gapo/tests-NAME-clang.jsonl: measurement
# only, its times are not the suite's. `cargo clean --profile dev` is not run
# here (the next timing run would rebuild from nothing); the caller cleans
# after the last run. Exits to files; nothing piped.
set -u
name="$1"; census="${2:-0}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/gapo"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
git rev-parse HEAD > "$out/tests-$name.head"
git status --porcelain --untracked-files=no > "$out/tests-$name.dirty"
if [ "$census" = 1 ]; then
  mkdir -p "$out/tests-$name-census-tmp" || exit 2
  export QCORE_CLANG="$here/clang-census.sh" QCORE_CLANG_REAL=/usr/lib/llvm-21/bin/clang \
    QCORE_CLANG_CENSUS_LOG="$out/tests-$name-clang.jsonl" \
    QCORE_CLANG_CENSUS_TMP="$out/tests-$name-census-tmp"
fi
flock "$lock" /usr/bin/env bash -c '
  start=$EPOCHREALTIME
  python3 scripts/run_test_binaries.py --log-dir "$1/tests-$2" > "$1/tests-$2.log" 2>&1
  rc=$?
  end=$EPOCHREALTIME
  echo "$rc" > "$1/tests-$2.exit"
  awk -v s="$start" -v e="$end" "BEGIN { printf \"%.3f\n\", e - s }" > "$1/tests-$2.seconds"
' _ "$out" "$name"
cat "$out/tests-$name.exit" "$out/tests-$name.seconds"
