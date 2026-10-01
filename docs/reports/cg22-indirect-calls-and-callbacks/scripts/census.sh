#!/usr/bin/env bash
# census.sh NAME BINARY -- the corpus arbiter's exact `pinned` command
# (scripts/run_pinned_audit.py: same arguments and environment), the system
# route and the tail-pause slice, run with an explicit binary: the census-only
# build (ADMIT = false) is measured while target/release holds another binary.
# Every exit is written to a file; nothing is piped.
set -u
name="$1"; binary="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg22"
inputs=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19
mkdir -p "$out/$name" || exit 2
cd "$repo" || exit 2
sha256sum "$binary" > "$out/$name.binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" QCORE_CLANG=/usr/bin/clang \
  QCORE_CHUNKIO_PACKAGE="$inputs/provider-packages-wave20/qcore.chunkio" \
  env -u QCORE_SOURCE_CONTRACTS "$binary" pinned --jobs 4 \
  "$inputs/c-gateway-wave-pin/fluent-bit-clang-source" \
  "$inputs/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
  "$out/$name/monkey-source.json" "$out/$name/resource-diagnostics.json" \
  > "$out/$name/audit.log" 2>&1
echo $? > "$out/$name.exit"
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
  bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "cg22-$name" \
  "$binary" \
  --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 4 \
  > "$out/$name-slice.log" 2>&1
echo $? > "$out/$name-slice.exit"
