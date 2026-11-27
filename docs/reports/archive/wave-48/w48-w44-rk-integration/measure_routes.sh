#!/usr/bin/env bash
# Measure the two Fluent Bit selections with the merged release binary.
set -u
repo="$(cd "$(dirname "$0")/../../.." && pwd)"
out="$repo/private/w48-w44-rk-integration/measure-combined"
binary="$repo/target/release/quirin-audit"
prepared_system="$repo/private/w45-fb/w46-external-callee-contracts/attempt-1"
prepared_slice="$repo/private/w47-tail/attempt-1/prep"
export QCORE_CROSS_UNIT_LINK_MAP="$repo/docs/reports/w44-cross-unit-summary/link-map.corpus.json"
export QCORE_CLANG=/usr/lib/llvm-21/bin/clang
mkdir -p "$out/system" "$out/slice" || exit 2
git -C "$repo" rev-parse HEAD > "$out/head"
sha256sum "$binary" > "$out/binary.sha256"
sha256sum "$QCORE_CROSS_UNIT_LINK_MAP" > "$out/link-map.sha256"

run_route() {
  local label="$1" checkout="$2" database="$3" selection="$4"
  export QCORE_NULL_PROPERTY="$out/$label-null.jsonl"
  export QCORE_OUTER_PROJECTION="$out/$label-outer.json"
  export QCORE_TYPED_PATH_PARITY="$out/$label-parity.jsonl"
  "$binary" current \
    --checkout "$checkout" \
    --profile-directory "$repo/contract/profiles/fluent-bit" \
    --compile-database "$database" \
    --selection "$selection" \
    --jobs 4 --report "$out/$label/resource-diagnostics.json" \
    > "$out/$label/console.log" 2> "$out/$label/stderr.log"
  local code=$?
  printf '%s\n' "$code" > "$out/$label.exit"
  printf '%s %s\n' "$label" "$code"
  return "$code"
}

run_route system \
  "$prepared_system/fluent-bit-clang-source" \
  "$prepared_system/fluent-bit-clang-production/compile_commands.json" \
  "$repo/contract/profiles/fluent-bit/system-selection.json" || exit 3
run_route slice \
  "$prepared_slice/fluent-bit-clang-source" \
  "$prepared_slice/fluent-bit-clang-production/compile_commands.json" \
  "$repo/contract/profiles/fluent-bit/tail-pause-selection.json" || exit 4
