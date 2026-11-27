#!/usr/bin/env bash
# final-measure.sh NAME -- release build, corpus arbiter, system route and
# tail-pause slice at this worktree's tip, in one acquisition of the shared
# heavy-step lock (the coordinator's serialisation, 2026-10-02). Every exit is
# written to a file; nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg30dft"
cd "$repo" || exit 2
CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 cargo build --release -p qcore-audit --bin quirin-audit \
  > "$out/build-$name.log" 2>&1
echo $? > "$out/build-$name.exit"
[ "$(cat "$out/build-$name.exit")" = 0 ] || exit 1
bash "$here/corpus.sh" "$name"
binary="$repo/target/release/quirin-audit"
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
  bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "cg30dft-$name" \
  "$binary" \
  --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 4 \
  > "$out/$name-slice.log" 2>&1
echo $? > "$out/$name-slice.exit"
