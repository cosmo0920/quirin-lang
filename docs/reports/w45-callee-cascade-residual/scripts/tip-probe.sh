#!/bin/bash
# tip-probe.sh NAME -- W45-CALLEE-CASCADE-RESIDUAL (from CHUNKIO-META-LEN-BOUNDS-CONTRACTS): the round-12 shared
# acceptance probe (scratchpad tip-contracts.sh) with this card's saved binary
# private/w45ccr/bin/NAME, reports into
# private/w45ccr/tip-NAME/. The scratch copy's
# mk_lib.c is the fix-branch tip's again afterwards, as the original leaves it.
S=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad; D=$S/monkey-tip-probe
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"; binary="$repo/private/w45ccr/bin/$name"
out="$repo/private/w45ccr/tip-$name"; mkdir -p "$out"
cd "$repo" || exit 2
INC=(); for i in monkey-build/mk_core/deps/libevent/include monkey-source/mk_core/deps/libevent/include monkey-build/include/monkey monkey-build/include monkey-source/include/monkey monkey-source/include monkey-source/deps/regex monkey-source/deps/flb_libco monkey-source/deps/rbtree monkey-source; do INC+=(-I "$D/$i"); done
run(){ env -u QCORE_BOUNDS_PROPERTY "$binary" contracts --source-root "$D/monkey-source" "${INC[@]}" -D MK_HAVE_ACCEPT4 -D MK_HAVE_BACKTRACE -D MK_HAVE_VALGRIND --std gnu17 --report "$1" mk_server/mk_lib.c > "$1.log" 2>&1; echo $? > "$1.exit"; echo "$1 exit $(cat "$1.exit")"; }
cp "$S/mk_lib_tip.c" "$D/monkey-source/mk_server/mk_lib.c"; run "$out/tip.json"
cp /home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/monkey/monkey-source/mk_server/mk_lib.c "$D/monkey-source/mk_server/mk_lib.c"; run "$out/master.json"
cp "$S/mk_lib_tip.c" "$D/monkey-source/mk_server/mk_lib.c"
