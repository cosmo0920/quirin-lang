#!/usr/bin/env bash
# build.sh TREE VARIANT -- build libfluent-bit.so for one tree and variant
# under /home/cosmo/Documents/GitHub/qcore-c/private/w47-dstop/attempt-1/.
#   TREE     90978ac6 (the W47-TAIL-PAUSE-BINDINGS export.tar, SHA-256
#            04a0fbee..., extracted fresh) or 046accf6 (a copy of the prepared
#            clone's working tree, .git and build output excluded)
#   VARIANT  debug (-O0 -g) or tsan (debug plus -DSANITIZE_THREAD=On)
# Never touches the user's checkouts or any Quirin tree; installs nothing.
# Every step's exit goes to its own file.
set -u
tree="$1"; variant="$2"
root=/home/cosmo/Documents/GitHub/qcore-c/private/w47-dstop/attempt-1
src="$root/src-$tree"
build="$root/build-$tree-$variant"
logs="$root/logs"
mkdir -p "$logs" || exit 2
tag="$tree-$variant"
if [ ! -d "$src" ]; then
  case "$tree" in
    90978ac6)
      tar=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/export.tar
      echo "04a0fbeeb6a74b37d82d6a2c2c956974f033196325d3e7d81e73df61e563eab8  $tar" | sha256sum -c - > "$logs/export-sha.log" 2>&1 || exit 3
      mkdir -p "$src" && tar -xf "$tar" -C "$src" || exit 3 ;;
    046accf6)
      clone=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
      mkdir -p "$src" && (cd "$clone" && tar --exclude=./.git --exclude=./build -cf - .) | tar -xf - -C "$src" || exit 3 ;;
    *) echo "unknown tree"; exit 2 ;;
  esac
fi
extra=()
case "$variant" in
  debug) ;;
  tsan) extra=(-DSANITIZE_THREAD=On) ;;
  *) echo "unknown variant"; exit 2 ;;
esac
mkdir -p "$build" || exit 2
cmake -S "$src" -B "$build" -G Ninja \
  -DCMAKE_C_COMPILER=/usr/lib/llvm-21/bin/clang \
  -DCMAKE_CXX_COMPILER=/usr/lib/llvm-21/bin/clang++ \
  -DCMAKE_BUILD_TYPE=Debug -DFLB_DEBUG=On -DFLB_RELEASE=Off \
  -DFLB_SHARED_LIB=On -DFLB_BINARY=Off -DFLB_EXAMPLES=Off \
  -DFLB_TESTS_RUNTIME=Off -DFLB_TESTS_INTERNAL=Off \
  -DCMAKE_C_FLAGS="-O0 -g -fno-omit-frame-pointer" \
  "${extra[@]}" > "$logs/configure-$tag.log" 2>&1
echo $? > "$logs/configure-$tag.exit"
[ "$(cat "$logs/configure-$tag.exit")" = 0 ] || exit 4
timeout 5400 cmake --build "$build" --target fluent-bit-shared -j 12 > "$logs/build-$tag.log" 2>&1
echo $? > "$logs/build-$tag.exit"
cat "$logs/build-$tag.exit"
