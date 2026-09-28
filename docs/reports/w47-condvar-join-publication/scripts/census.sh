#!/usr/bin/env bash
# census.sh OUT_DIR -- the condvar/join census (card
# W47-CONDVAR-JOIN-PUBLICATION, rule 10q) over the in_tail runtime test
# binary's sources at 90978ac6 (the W47-TAIL-PAUSE-BINDINGS export, synthetic
# commit 34155955) and at 90978ac6~1 (the same tree with the previous test,
# blob-identical to 046accf6's tests/runtime/in_tail.c). Read-only on both;
# the file set is the loop-owner census's.
set -u
out="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
export_root=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source
previous_test=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source/tests/runtime/in_tail.c
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
CARGO_INCREMENTAL=0 cargo build -q -p quirin-source-analysis --example condvar_join_census > "$out/build.log" 2>&1
echo $? > "$out/build.exit"
bin="$repo/target/debug/examples/condvar_join_census"
paths=(src plugins lib/monkey include tests/runtime/in_tail.c '!src/fluent-bit.c')
"$bin" "$export_root" - "${paths[@]}" > "$out/fixed.json" 2> "$out/fixed.stderr"
echo $? > "$out/fixed.exit"
"$bin" "$export_root" "$previous_test" "${paths[@]}" > "$out/previous.json" 2> "$out/previous.stderr"
echo $? > "$out/previous.exit"
sha256sum "$previous_test" "$export_root/tests/runtime/in_tail.c" > "$out/tests.sha256"
