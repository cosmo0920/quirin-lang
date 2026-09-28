#!/usr/bin/env bash
# census.sh OUT_DIR -- the thread-entry census over the in_tail runtime test
# binary's sources at 90978ac6 (the W47-TAIL-PAUSE-BINDINGS export, synthetic
# commit 34155955) and at 90978ac6~1 (the same tree with the previous test),
# as W47-LOOP-OWNER-CONTRACTS ran it, twice: the raw reading (every
# conditional arm) and the configured reading under the export's recorded
# production build (its compile_commands.json and generated headers, macros
# prefixed FLB_ owned by
# the build configuration). Read-only on every tree.
set -u
out="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
prep=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep
export_root="$prep/fluent-bit-clang-source"
database="$prep/fluent-bit-clang-production/compile_commands.json"
previous_test=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source/tests/runtime/in_tail.c
module="$repo/providers/quirin-provider-fluent-bit/contracts/fluent_bit/loop_owner.qv"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
CARGO_INCREMENTAL=0 cargo build -q -p quirin-source-analysis --example loop_owner_census > "$out/build.log" 2>&1
echo $? > "$out/build.exit"
bin="$repo/target/debug/examples/loop_owner_census"
paths=(src plugins lib/monkey include tests/runtime/in_tail.c '!src/fluent-bit.c')
configured=("--compile-commands=$database" "--generated-root=$prep/fluent-bit-clang-production" --owned-prefix=FLB_)
"$bin" "$module" "$export_root" - "${paths[@]}" > "$out/fixed.json" 2> "$out/fixed.stderr"
echo $? > "$out/fixed.exit"
"$bin" "$module" "$export_root" "$previous_test" "${paths[@]}" > "$out/previous.json" 2> "$out/previous.stderr"
echo $? > "$out/previous.exit"
"$bin" "${configured[@]}" "$module" "$export_root" - "${paths[@]}" > "$out/fixed-configured.json" 2> "$out/fixed-configured.stderr"
echo $? > "$out/fixed-configured.exit"
"$bin" "${configured[@]}" "$module" "$export_root" "$previous_test" "${paths[@]}" > "$out/previous-configured.json" 2> "$out/previous-configured.stderr"
echo $? > "$out/previous-configured.exit"
sha256sum "$previous_test" "$export_root/tests/runtime/in_tail.c" "$database" > "$out/inputs.sha256"
