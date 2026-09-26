#!/usr/bin/env bash
# Run the Fluent Bit system slice with `quirin-audit current` over a prepared attempt,
# from the repository root.
# usage: run-current.sh ATTEMPT RUN-NAME BINARY [EXTRA current ARGUMENTS...]
set -u
attempt="$1"; run="$2"; binary="$3"; shift 3
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-contract-revision-mode/$attempt
mkdir "$out/$run" || exit 2
cd "$repo" || exit 2
QCORE_CLANG=/usr/lib/llvm-21/bin/clang timeout 3600 "$binary" current \
  --checkout "$out/fluent-bit-clang-source" \
  --profile-directory "$repo/contract/profiles/fluent-bit" \
  --compile-database "$out/fluent-bit-clang-production/compile_commands.json" \
  --selection contract/profiles/fluent-bit/system-selection.json \
  --report "$out/$run/resource-diagnostics.json" \
  --jobs 8 \
  "$@" \
  > "$out/$run/console.log" 2> "$out/$run/stderr.log"
echo $? > "$out/$run/exit"
cat "$out/$run/exit"
