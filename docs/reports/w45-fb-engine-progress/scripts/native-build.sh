#!/usr/bin/env bash
# Configure and build an unmodified Debug Fluent Bit binary for the native runs,
# out of tree, from the clone `quirin-audit prepare` made (attempt-1).
# usage: native-build.sh ATTEMPT
# The source tree is only read. Afterwards `prepare-attempt.sh ATTEMPT BINARY
# REVISION -recheck` re-verifies the clone (HEAD = pin, no tracked change).
set -u
attempt="$1"
root=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-engine-progress/$attempt
src="$root/fluent-bit-clang-source"
build="$root/native-debug"
logs="$root/native-logs"
mkdir -p "$logs"
mkdir "$build" || exit 2
configure=(cmake -G Ninja -S "$src" -B "$build"
  -DCMAKE_BUILD_TYPE=Debug
  -DCMAKE_C_COMPILER=/usr/lib/llvm-21/bin/clang -DCMAKE_CXX_COMPILER=/usr/lib/llvm-21/bin/clang++
  -DFLB_EXAMPLES=Off -DFLB_SHARED_LIB=Off -DFLB_JEMALLOC=Off -DFLB_WASM=Off -DFLB_LUAJIT=Off
  -DFLB_STREAM_PROCESSOR=Off -DFLB_TESTS_INTERNAL=Off -DFLB_TESTS_RUNTIME=Off)
printf '%q ' "${configure[@]}" > "$logs/configure.cmd"
timeout 1800 "${configure[@]}" > "$logs/configure.log" 2>&1
echo $? > "$logs/configure.exit"
[ "$(cat "$logs/configure.exit")" = 0 ] || { echo "configure exit $(cat "$logs/configure.exit")"; exit 1; }
timeout 3600 ninja -C "$build" -j 8 fluent-bit > "$logs/build.log" 2>&1
echo $? > "$logs/build.exit"
echo "build exit $(cat "$logs/build.exit")"
sha256sum "$build/bin/fluent-bit" > "$logs/binary.sha256"
"$build/bin/fluent-bit" --version > "$logs/binary-version.txt" 2>&1
cat "$logs/binary.sha256" "$logs/binary-version.txt"
