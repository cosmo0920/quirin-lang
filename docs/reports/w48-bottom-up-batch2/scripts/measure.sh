#!/usr/bin/env bash
# Fresh private measurements. Parent grants the heavy-run slot first.
set -u
mode="${1:?corpus|system|slice}" run="${2:?fresh run name}" map="${3:?relative link map}"
repo="$(cd "$(dirname "$0")/../../../.." && pwd)"
out="$repo/private/w48-bottom-up-batch2/$run"
[ ! -e "$out" ] || exit 2
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
binary="$repo/target/release/quirin-audit"
export QCORE_CROSS_UNIT_LINK_MAP="$repo/$map"
export QCORE_CLANG=/usr/lib/llvm-21/bin/clang
git rev-parse HEAD > "$out/head"
sha256sum "$binary" > "$out/binary.sha256"
sha256sum "$QCORE_CROSS_UNIT_LINK_MAP" > "$out/link-map.sha256"
case "$mode" in
corpus)
  python3 scripts/run_pinned_audit.py "$out/corpus" 66910c10 \
    --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 4 \
    > "$out/console.log" 2>&1
  code=$?;;
system|slice)
  if [ "$mode" = system ]; then
    prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
    selection=system-selection.json
  else
    prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep
    selection=tail-pause-selection.json
  fi
  "$binary" current \
    --checkout "$prepared/fluent-bit-clang-source" \
    --profile-directory "$repo/contract/profiles/fluent-bit" \
    --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
    --selection "$repo/contract/profiles/fluent-bit/$selection" \
    --jobs 4 --report "$out/resource-diagnostics.json" \
    > "$out/console.log" 2> "$out/stderr.log"
  code=$?;;
*) exit 2;;
esac
printf '%s\n' "$code" > "$out/$mode.exit"
exit "$code"
