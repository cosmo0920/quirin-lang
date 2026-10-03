#!/usr/bin/env bash
# tip-probe.sh NAME -- CAST-FORMALIZATION: the shared mk_vhost_set acceptance
# probe (the integrator's tip-contracts.sh) on this worktree's private copy of
# the scratch Monkey tree (private/castf/tip-probe), with this worktree's
# release quirin-audit: the contracts-only route over mk_server/mk_lib.c at the
# tip (21a2c190's fix line) and at master (the corpus copy). Exits to files.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
D="$repo/private/castf/tip-probe"
out="$D/out-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
INC=()
for i in monkey-build/mk_core/deps/libevent/include monkey-source/mk_core/deps/libevent/include \
  monkey-build/include/monkey monkey-build/include monkey-source/include/monkey \
  monkey-source/include monkey-source/deps/regex monkey-source/deps/flb_libco \
  monkey-source/deps/rbtree monkey-source; do
  INC+=(-I "$D/$i")
done
run() {
  target/release/quirin-audit contracts --source-root "$D/monkey-source" "${INC[@]}" \
    -D MK_HAVE_ACCEPT4 -D MK_HAVE_BACKTRACE -D MK_HAVE_VALGRIND --std gnu17 \
    --report "$1" mk_server/mk_lib.c > "$1.log" 2>&1
  echo $? > "$1.exit"
}
cp "$D/mk_lib_tip.c" "$D/monkey-source/mk_server/mk_lib.c"
run "$out/tip.json"
cp /home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/monkey/monkey-source/mk_server/mk_lib.c \
  "$D/monkey-source/mk_server/mk_lib.c"
run "$out/master.json"
cp "$D/mk_lib_tip.c" "$D/monkey-source/mk_server/mk_lib.c"
python3 "$here/tip_rows.py" "$out/tip.json" "$out/master.json" > "$out/rows.txt"
echo $? > "$out/rows.exit"
