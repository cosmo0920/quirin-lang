#!/usr/bin/env bash
# slice.sh NAME -- re-run only the tail-pause slice of measure.sh with this
# worktree's release quirin-audit into the shared prepared attempt as
# cg23rp-NAME (the commands of docs/reports/w47-tail-pause-bindings/scripts/
# run-current.sh, inlined). Used when a slice run fails for a host reason.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
binary="${BINARY:-$repo/target/release/quirin-audit}"
mkdir "$out/cg23rp-$name" || exit 2
cd "$repo" || exit 2
QCORE_TYPED_PATH_PARITY="$repo/private/cg23rp/$name-slice-parity.jsonl" \
  QCORE_CLANG=/usr/lib/llvm-21/bin/clang timeout 3600 "$binary" current \
  --checkout "$out/prep/fluent-bit-clang-source" \
  --profile-directory "$repo/contract/profiles/fluent-bit" \
  --compile-database "$out/prep/fluent-bit-clang-production/compile_commands.json" \
  --report "$out/cg23rp-$name/resource-diagnostics.json" \
  --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 4 \
  > "$out/cg23rp-$name/console.log" 2> "$out/cg23rp-$name/stderr.log"
echo $? > "$out/cg23rp-$name/exit"
cat "$out/cg23rp-$name/exit"
