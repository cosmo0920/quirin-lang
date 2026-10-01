#!/usr/bin/env bash
# measure_routes.sh NAME BINARY TREE -- CG25-RESOURCE-KINDS: the system route
# and the tail-pause slice with BINARY, reading the contract profiles and the
# provider modules of TREE (a checkout or an extracted `git archive` of one).
#
# measure.sh reads the profiles of this worktree, so a run whose contract
# texts changed under it measures a mixture: the card's first BEFORE did
# (private/cg25rk/before-system, before-slice: both rejected the edited
# `libc/dlfcn.qv` with exit 1 and are discarded). This script pins the tree.
# Outputs: private/cg25rk/NAME-system and the shared slice attempt's
# cg25rk-NAME. Every exit is written to a file; nothing is piped.
set -u
name="$1"; binary="$2"; tree="$3"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg25rk"
mkdir -p "$out" || exit 2
sha256sum "$binary" > "$out/$name.binary.sha256"
prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
mkdir "$out/$name-system" || exit 2
cd "$tree" || exit 2
QCORE_TYPED_PATH_PARITY="$out/$name-system-parity.jsonl" QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
  "$binary" current \
  --checkout "$prepared/fluent-bit-clang-source" \
  --profile-directory "$tree/contract/profiles/fluent-bit" \
  --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
  --selection "$tree/contract/profiles/fluent-bit/system-selection.json" \
  --jobs 4 --report "$out/$name-system/resource-diagnostics.json" \
  > "$out/$name-system/console.log" 2> "$out/$name-system/stderr.log"
echo $? > "$out/$name-system.exit"
slices=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
mkdir "$slices/cg25rk-$name" || exit 2
QCORE_TYPED_PATH_PARITY="$out/$name-slice-parity.jsonl" QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
  timeout 3600 "$binary" current \
  --checkout "$slices/prep/fluent-bit-clang-source" \
  --profile-directory "$tree/contract/profiles/fluent-bit" \
  --compile-database "$slices/prep/fluent-bit-clang-production/compile_commands.json" \
  --report "$slices/cg25rk-$name/resource-diagnostics.json" \
  --selection "$tree/contract/profiles/fluent-bit/tail-pause-selection.json" --jobs 4 \
  > "$slices/cg25rk-$name/console.log" 2> "$slices/cg25rk-$name/stderr.log"
echo $? > "$slices/cg25rk-$name/exit"
cp "$slices/cg25rk-$name/exit" "$out/$name-slice.exit"
