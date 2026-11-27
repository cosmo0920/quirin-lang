#!/usr/bin/env bash
# compare_trees.sh OUT -- byte comparison of the files on the observed paths
# between the 90978ac6 export and the 046accf6 prepared clone (read-only on
# both). Writes OUT/compare.txt: one line per file, "same" or "differs".
set -u
out="$1"
a=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source
b=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
mkdir -p "$out" || exit 2
: > "$out/compare.txt"
for f in src/flb_lib.c src/flb_engine.c src/flb_fips.c src/flb_input.c \
         src/flb_config.c src/flb_utils.c include/fluent-bit/flb_lib.h \
         include/fluent-bit/flb_config.h lib/monkey/mk_core/mk_utils.c \
         plugins/in_tail/tail_config.c plugins/in_tail/tail.c \
         plugins/in_dummy/in_dummy.c plugins/out_null/null.c \
         plugins/in_lib/in_lib.c plugins/out_lib/out_lib.c; do
  if cmp -s -- "$a/$f" "$b/$f"; then
    echo "same $f $(sha256sum -- "$a/$f" | cut -c1-64)" >> "$out/compare.txt"
  else
    echo "differs $f" >> "$out/compare.txt"
  fi
done
cat "$out/compare.txt"
