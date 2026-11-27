#!/usr/bin/env bash
# runtime_tests.sh OUT_DIR BINARY [TREE] -- the census's lifecycle reading of
# every Fluent Bit runtime test file (card W47-FLB-LIB-STRICT-PAIRING): one
# census per tests/runtime/*.c at TREE (default: the 90978ac6 export), with
# the same library file set as census.sh and that one test file as the test
# roots, under this worktree's lifecycle module. Sizes the caller candidates
# outside in_tail.c; the release verdicts of these runs are not read (their
# retention premise P-RETAIN names in_tail's code only). Read-only on the
# tree; every exit to a file; 4 files at a time.
set -u
out="$1"
bin="$2"
tree="${3:-/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
module="$repo/providers/quirin-provider-fluent-bit/contracts/fluent_bit/lib_engine_thread.qv"
mkdir -p "$out" || exit 2
sha256sum "$bin" "$module" > "$out/inputs.sha256"
ls "$tree/tests/runtime/" > "$out/listing.txt"
one() {
  name="$(basename "$1" .c)"
  "$bin" "--lifecycle-module=$module" "$tree" - src plugins lib/monkey include \
    "tests/runtime/$name.c" '!src/fluent-bit.c' > "$out/$name.json" 2> "$out/$name.stderr"
  echo $? > "$out/$name.exit"
}
export -f one
export bin module tree out
find "$tree/tests/runtime" -maxdepth 1 -name '*.c' -printf '%f\n' | sort > "$out/files.txt"
xargs -P 4 -I{} bash -c 'one "$@"' _ {} < "$out/files.txt"
echo done > "$out/done"
