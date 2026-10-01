#!/usr/bin/env bash
# system-only.sh NAME BINARY -- CG17-LAYOUT-DERIVED-EXTENTS: the system route of
# measure.sh alone (the BEFORE run's exit record of this step was overwritten
# when measure.sh was edited while it ran; this re-run is the clean record).
set -u
name="$1"; binary="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg17le"
prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
mkdir -p "$out/$name-system" || exit 2
cd "$repo" || exit 2
sha256sum "$binary" > "$out/$name-system.binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/$name-system-parity.jsonl" QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
  "$binary" current \
  --checkout "$prepared/fluent-bit-clang-source" \
  --profile-directory "$repo/contract/profiles/fluent-bit" \
  --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
  --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
  --jobs 4 --report "$out/$name-system/resource-diagnostics.json" \
  > "$out/$name-system/console.log" 2> "$out/$name-system/stderr.log"
echo $? > "$out/$name-system.exit"
