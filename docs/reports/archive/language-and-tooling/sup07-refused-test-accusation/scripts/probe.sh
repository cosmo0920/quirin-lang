#!/usr/bin/env bash
# probe.sh NAME -- the shared acceptance probe (the integrator's tip-contracts.sh)
# on a private copy of its scratch tree: `quirin-audit contracts` over Monkey's
# mk_server/mk_lib.c at the fix tip (21a2c190) and at master (the corpus copy),
# with this worktree's release binary. Writes private/sup07/probe-NAME/{tip,master}.json.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
scratch=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad
lock="$scratch/heavy.lock"
out="$repo/private/sup07/probe-$name"
tree="$out/tree"
mkdir -p "$out" || exit 2
cp -r "$scratch/monkey-tip-probe/monkey-build" "$scratch/monkey-tip-probe/monkey-source" "$out/" || exit 2
mkdir -p "$tree" && mv "$out/monkey-build" "$out/monkey-source" "$tree/" || exit 2
cd "$repo" || exit 2
inc=()
for i in monkey-build/mk_core/deps/libevent/include monkey-source/mk_core/deps/libevent/include \
  monkey-build/include/monkey monkey-build/include monkey-source/include/monkey monkey-source/include \
  monkey-source/deps/regex monkey-source/deps/flb_libco monkey-source/deps/rbtree monkey-source; do
  inc+=(-I "$tree/$i")
done
run() {
  flock "$lock" target/release/quirin-audit contracts --source-root "$tree/monkey-source" "${inc[@]}" \
    -D MK_HAVE_ACCEPT4 -D MK_HAVE_BACKTRACE -D MK_HAVE_VALGRIND --std gnu17 --report "$1" \
    mk_server/mk_lib.c > "$1.log" 2>&1
  echo $? > "$1.exit"
}
cp "$scratch/mk_lib_tip.c" "$tree/monkey-source/mk_server/mk_lib.c" || exit 2
run "$out/tip.json"
cp /home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/monkey/monkey-source/mk_server/mk_lib.c \
  "$tree/monkey-source/mk_server/mk_lib.c" || exit 2
run "$out/master.json"
