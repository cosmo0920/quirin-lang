#!/bin/bash
# tip-probe.sh NAME BINARY -- the round-12 shared acceptance probe
# (`mk_vhost_set` at the tip of Monkey's cosmo0920-missing-va_end-on-null,
# 21a2c190) through the contracts-only route, on this card's private copy of
# the integrator's scratch probe tree (private/cg26r/monkey-tip-probe; the
# integrator's tip-contracts.sh with its paths moved into this worktree). Runs
# from this worktree, so the route reads this tree's contract modules. Writes
# private/cg26r/tip-NAME.json (the fixed mk_lib.c) and master-NAME.json (the
# corpus copy), each with its .log and .exit; the probe tree's mk_lib.c is left
# at the tip copy, as the integrator's script leaves it.
set -u
name="$1"
binary="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
D="$repo/private/cg26r/monkey-tip-probe"
O="$repo/private/cg26r"
cd "$repo" || exit 2
INC=()
for i in monkey-build/mk_core/deps/libevent/include monkey-source/mk_core/deps/libevent/include monkey-build/include/monkey monkey-build/include monkey-source/include/monkey monkey-source/include monkey-source/deps/regex monkey-source/deps/flb_libco monkey-source/deps/rbtree monkey-source; do
  INC+=(-I "$D/$i")
done
run() {
  "$binary" contracts --source-root "$D/monkey-source" "${INC[@]}" -D MK_HAVE_ACCEPT4 -D MK_HAVE_BACKTRACE -D MK_HAVE_VALGRIND --std gnu17 --report "$1" mk_server/mk_lib.c > "$1.log" 2>&1
  echo $? > "$1.exit"
  echo "$1 exit $(cat "$1.exit")"
}
cp "$O/mk_lib_tip.c" "$D/monkey-source/mk_server/mk_lib.c"
run "$O/tip-$name.json"
cp /home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/monkey/monkey-source/mk_server/mk_lib.c "$D/monkey-source/mk_server/mk_lib.c"
run "$O/master-$name.json"
cp "$O/mk_lib_tip.c" "$D/monkey-source/mk_server/mk_lib.c"
