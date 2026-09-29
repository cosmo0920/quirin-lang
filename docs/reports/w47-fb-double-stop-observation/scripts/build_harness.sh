#!/usr/bin/env bash
# build_harness.sh TREE VARIANT -- copy harness.c and joinlog.c into the
# attempt directory and build them there against build-TREE-VARIANT, with the
# -I/-D flags Ninja uses for src/flb_lib.c in that build (so the harness reads
# the same struct layout the library was compiled with).
set -u
tree="$1"; variant="$2"
here="$(cd "$(dirname "$0")" && pwd)"
root=/home/cosmo/Documents/GitHub/qcore-c/private/w47-dstop/attempt-1
build="$root/build-$tree-$variant"
out="$root/harness-$tree-$variant"
logs="$root/logs"
cc=/usr/lib/llvm-21/bin/clang
mkdir -p "$out" "$logs" || exit 2
cp -- "$here/harness.c" "$here/joinlog.c" "$out/" || exit 2
flags=$(ninja -C "$build" -t commands fluent-bit-shared | grep ' -c [^ ]*/src/flb_lib.c' | head -1 \
        | tr ' ' '\n' | grep -E '^-(I|D|isystem)' | tr '\n' ' ')
[ -n "$flags" ] || exit 3
san=()
[ "$variant" = tsan ] && san=(-fsanitize=thread)
# shellcheck disable=SC2086
"$cc" -O0 -g -fno-omit-frame-pointer "${san[@]}" $flags -o "$out/harness" "$out/harness.c" \
  -L"$build/lib" -lfluent-bit -Wl,-rpath,"$build/lib" -lpthread > "$logs/harness-$tree-$variant.log" 2>&1
echo $? > "$logs/harness-$tree-$variant.exit"
"$cc" -O0 -g -shared -fPIC -o "$out/joinlog.so" "$out/joinlog.c" -ldl > "$logs/joinlog-$tree-$variant.log" 2>&1
echo $? > "$logs/joinlog-$tree-$variant.exit"
echo "harness $(cat "$logs/harness-$tree-$variant.exit") joinlog $(cat "$logs/joinlog-$tree-$variant.exit")"
sha256sum -- "$out/harness" "$out/joinlog.so" "$build/lib/libfluent-bit.so" > "$out/sha256.txt"
cat "$out/sha256.txt"
