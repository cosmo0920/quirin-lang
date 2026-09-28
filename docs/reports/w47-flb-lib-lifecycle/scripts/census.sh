#!/usr/bin/env bash
# census.sh OUT_DIR -- the condvar/join census under the declared lifecycle
# obligations (card W47-FLB-LIB-LIFECYCLE, rule 10q as amended): the rule 10q
# census's file set (docs/reports/w47-condvar-join-publication/scripts/census.sh)
# with --lifecycle-module naming this worktree's
# fluent_bit/lib_engine_thread.qv, over
#   fixed.json     the 90978ac6 export (W47-TAIL-PAUSE-BINDINGS, synthetic
#                  commit 34155955), the fixed test;
#   previous.json  the same tree with the previous test (90978ac6~1,
#                  blob-identical to 046accf6's tests/runtime/in_tail.c);
#   system.json    the 046accf6 prepared clone (its own test file);
#   corpus.json    the 66910c10 corpus source (read-only; its own test file).
# Read-only on every tree; every exit to a file.
set -u
out="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
export_root=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source
system_root=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
corpus_root=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
previous_test="$system_root/tests/runtime/in_tail.c"
module="$repo/providers/quirin-provider-fluent-bit/contracts/fluent_bit/lib_engine_thread.qv"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
CARGO_INCREMENTAL=0 cargo build -q -p quirin-source-analysis --example condvar_join_census > "$out/build.log" 2>&1
echo $? > "$out/build.exit"
bin="$repo/target/debug/examples/condvar_join_census"
paths=(src plugins lib/monkey include tests/runtime/in_tail.c '!src/fluent-bit.c')
"$bin" "--lifecycle-module=$module" "$export_root" - "${paths[@]}" > "$out/fixed.json" 2> "$out/fixed.stderr"
echo $? > "$out/fixed.exit"
"$bin" "--lifecycle-module=$module" "$export_root" "$previous_test" "${paths[@]}" > "$out/previous.json" 2> "$out/previous.stderr"
echo $? > "$out/previous.exit"
"$bin" "--lifecycle-module=$module" "$system_root" - "${paths[@]}" > "$out/system.json" 2> "$out/system.stderr"
echo $? > "$out/system.exit"
"$bin" "--lifecycle-module=$module" "$corpus_root" - "${paths[@]}" > "$out/corpus.json" 2> "$out/corpus.stderr"
echo $? > "$out/corpus.exit"
sha256sum "$previous_test" "$export_root/tests/runtime/in_tail.c" "$corpus_root/tests/runtime/in_tail.c" "$module" > "$out/inputs.sha256"
