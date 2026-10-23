#!/bin/sh
# Focused contracts-only analysis of the two scheduler units, with the corpus
# Monkey build's flags, over the Fluent Bit 66910c10 root (where the provider
# pins name lib/monkey/... sites). Evidence for iteration only; the arbiter is
# the pinned corpus route.
# Usage: focus_units.sh BINARY OUTDIR
set -u
binary=$1
out=$2
mkdir -p "$out"
fb=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
mb=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/monkey/monkey-build
ms=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/monkey/monkey-source
"$binary" contracts --source-root "$fb" \
  "$fb/lib/monkey/mk_server/mk_scheduler.c" "$fb/lib/monkey/mk_server/mk_http_thread.c" \
  -I "$mb/mk_core/deps/libevent/include" -I "$ms/mk_core/deps/libevent/include" \
  -I "$mb/include/monkey" -I "$mb/include" \
  -I "$fb/lib/monkey/include/monkey" -I "$fb/lib/monkey/include" \
  -I "$ms/deps/regex" -I "$ms/deps/flb_libco" -I "$ms/deps/rbtree" -I "$ms/." \
  -D MK_HAVE_ACCEPT4 -D MK_HAVE_BACKTRACE -D MK_HAVE_VALGRIND --std gnu17 \
  --report "$out/resource-diagnostics.json" >"$out/console.log" 2>&1
echo $? >"$out/exit"
