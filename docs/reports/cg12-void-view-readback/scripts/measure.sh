#!/usr/bin/env bash
# measure.sh NAME -- CG12-VOID-VIEW-READBACK's corpus arbiter, system route and
# tail-pause slice with this worktree's release quirin-audit, each into a fresh
# directory (corpus/system under private/cg12/NAME*, the slice under the shared
# prepared attempt as cg12-NAME). Adapted from
# docs/reports/cg10-bound-untracked-roots/scripts/measure.sh with the round-7
# shared-host limits (--jobs 4, CARGO_BUILD_JOBS=6). Every exit is written to a
# file; nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg12"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
# SKIP_BUILD=1 measures the release binary already in target/release.
if [ "${SKIP_BUILD:-0}" != 1 ]; then
  CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 cargo build --release -p qcore-audit --bin quirin-audit \
    > "$out/build-$name.log" 2>&1
  echo $? > "$out/build-$name.exit"
fi
binary="$repo/target/release/quirin-audit"
sha256sum "$binary" > "$out/$name.binary.sha256"
git -C "$repo" rev-parse HEAD > "$out/$name.head"
QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" python3 scripts/run_pinned_audit.py \
  "$out/$name" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 4 > "$out/$name.log" 2>&1
echo $? > "$out/$name.exit"
if [ "${CORPUS_ONLY:-0}" = 1 ]; then
  exit 0
fi
prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
mkdir -p "$out/$name-system" || exit 2
QCORE_TYPED_PATH_PARITY="$out/$name-system-parity.jsonl" QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
  "$binary" current \
  --checkout "$prepared/fluent-bit-clang-source" \
  --profile-directory "$repo/contract/profiles/fluent-bit" \
  --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
  --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
  --jobs 4 --report "$out/$name-system/resource-diagnostics.json" \
  > "$out/$name-system/console.log" 2> "$out/$name-system/stderr.log"
echo $? > "$out/$name-system.exit"
QCORE_TYPED_PATH_PARITY="$out/$name-slice-parity.jsonl" \
  bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "cg12-$name" \
  "$binary" \
  --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 4 \
  > "$out/$name-slice.log" 2>&1
echo $? > "$out/$name-slice.exit"
