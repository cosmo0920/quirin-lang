#!/bin/bash
# tip-probe.sh NAME -- NULL-SAFETY-PROPERTY (from DEEP-AST-TRANSPORTS-AND-WALL-TIME's
# scripts/tip-probe.sh, the round-12 shared acceptance probe): the scratchpad's
# probe copy (monkey-tip-probe/, mk_lib_tip.c) was gone when this card ran, so
# it is rebuilt here, privately, from the read-only corpus copy of Monkey
# (build/cycle19/c-gateway-wave-pin/monkey): the master mk_lib.c, and the tip
# as the master plus the fix-branch's one line (`va_end(va);` before the
# `return -1;` of the `!value` arm in mk_vhost_set, the round-12 brief's
# "only difference"). Runs `quirin-audit contracts` with the saved binary
# private/nsp/bin/NAME on both, reports into private/nsp/tip-NAME/, under the
# shared heavy lock, with QCORE_NULL_PROPERTY also set (null rows beside each).
S=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
name="$1"; binary="$repo/private/nsp/bin/$name"
M=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/monkey
D="$repo/private/nsp/monkey-tip-probe"
out="$repo/private/nsp/tip-$name"; mkdir -p "$out"
cd "$repo" || exit 2
if [ ! -d "$D/monkey-source" ]; then
  mkdir -p "$D" && cp -r "$M/monkey-source" "$M/monkey-build" "$D/" || exit 2
  python3 - "$D/monkey-source/mk_server/mk_lib.c" "$D/mk_lib_tip.c" <<'EOF' || exit 2
import sys
text = open(sys.argv[1]).read()
old = "        if (!value) {\n            /* Wrong parameter */\n            return -1;\n        }\n"
new = "        if (!value) {\n            /* Wrong parameter */\n            va_end(va);\n            return -1;\n        }\n"
assert text.count(old) == 1
open(sys.argv[2], "w").write(text.replace(old, new))
EOF
fi
exec 9> "$S/heavy.lock"; flock 9
INC=(); for i in monkey-build/mk_core/deps/libevent/include monkey-source/mk_core/deps/libevent/include monkey-build/include/monkey monkey-build/include monkey-source/include/monkey monkey-source/include monkey-source/deps/regex monkey-source/deps/flb_libco monkey-source/deps/rbtree monkey-source; do INC+=(-I "$D/$i"); done
run(){ QCORE_NULL_PROPERTY="$1.null.jsonl" "$binary" contracts --source-root "$D/monkey-source" "${INC[@]}" -D MK_HAVE_ACCEPT4 -D MK_HAVE_BACKTRACE -D MK_HAVE_VALGRIND --std gnu17 --report "$1" mk_server/mk_lib.c > "$1.log" 2>&1; echo $? > "$1.exit"; echo "$1 exit $(cat "$1.exit")"; }
cp "$D/mk_lib_tip.c" "$D/monkey-source/mk_server/mk_lib.c"; run "$out/tip.json"
cp "$M/monkey-source/mk_server/mk_lib.c" "$D/monkey-source/mk_server/mk_lib.c"; run "$out/master.json"
cp "$D/mk_lib_tip.c" "$D/monkey-source/mk_server/mk_lib.c"
