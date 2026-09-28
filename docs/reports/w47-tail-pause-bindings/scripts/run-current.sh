#!/usr/bin/env bash
# Run `quirin-audit current` over a prepared W47-TAIL-PAUSE-BINDINGS attempt,
# from the repository root (adapted from w45-fb-backlog-custody/scripts).
# usage: run-current.sh ATTEMPT RUN-NAME BINARY [EXTRA current ARGUMENTS...]
#   tail-pause slice: ... --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 8
set -u
attempt="$1"; run="$2"; binary="$3"; shift 3
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/$attempt
mkdir "$out/$run" || exit 2
cd "$repo" || exit 2
QCORE_CLANG=/usr/lib/llvm-21/bin/clang timeout 3600 "$binary" current \
  --checkout "$out/prep/fluent-bit-clang-source" \
  --profile-directory "$repo/contract/profiles/fluent-bit" \
  --compile-database "$out/prep/fluent-bit-clang-production/compile_commands.json" \
  --report "$out/$run/resource-diagnostics.json" \
  "$@" \
  > "$out/$run/console.log" 2> "$out/$run/stderr.log"
echo $? > "$out/$run/exit"
cat "$out/$run/exit"
