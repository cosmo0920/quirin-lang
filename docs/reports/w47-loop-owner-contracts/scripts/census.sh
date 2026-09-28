#!/usr/bin/env bash
# census.sh OUT_DIR -- the thread-entry census over the in_tail runtime test
# binary's sources at 90978ac6 (the W47-TAIL-PAUSE-BINDINGS export, synthetic
# commit 34155955) and at 90978ac6~1 (the same tree with the previous test,
# blob-identical to 046accf6's tests/runtime/in_tail.c). Read-only on both.
set -u
out="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
export_root=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source
previous_test=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source/tests/runtime/in_tail.c
module="$repo/providers/quirin-provider-fluent-bit/contracts/fluent_bit/loop_owner.qv"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
CARGO_INCREMENTAL=0 cargo build -q -p qcore-c-gateway --example loop_owner_census > "$out/build.log" 2>&1
echo $? > "$out/build.exit"
bin="$repo/target/debug/examples/loop_owner_census"
# The test binary: the library, every plugin, Monkey, the public headers and
# the test itself; src/fluent-bit.c (the service's own main) is not linked in.
paths=(src plugins lib/monkey include tests/runtime/in_tail.c '!src/fluent-bit.c')
"$bin" "$module" "$export_root" - "${paths[@]}" > "$out/fixed.json" 2> "$out/fixed.stderr"
echo $? > "$out/fixed.exit"
"$bin" "$module" "$export_root" "$previous_test" "${paths[@]}" > "$out/previous.json" 2> "$out/previous.stderr"
echo $? > "$out/previous.exit"
sha256sum "$previous_test" "$export_root/tests/runtime/in_tail.c" > "$out/tests.sha256"
