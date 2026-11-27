#!/bin/bash
# Does a second CMake configure of a fresh Fluent Bit build tree change its
# compile database? Configures RUN's source into a scratch build directory
# with prepare's exact configure arguments, twice, keeping both databases.
# Usage: configure_twice.sh WORKTREE RUN
set -euo pipefail
worktree=$1
run=$2
source="$worktree/private/link-prepare/$run/fluent-bit-clang-source"
build="$worktree/private/link-prepare/$run-configure-twice"
mkdir -p "$build"
configure() {
  cmake -S "$source" -B "$build" \
    -DCMAKE_C_COMPILER=/usr/lib/llvm-21/bin/clang \
    -DCMAKE_CXX_COMPILER=/usr/lib/llvm-21/bin/clang++ \
    -DCMAKE_C_STANDARD=17 '-DCMAKE_C_FLAGS:STRING=-std=gnu17' \
    -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -DFLB_DEV=ON -DFLB_EXAMPLES=ON > "$build/configure-$1.log" 2>&1
  cp "$build/compile_commands.json" "$build/compile_commands.$1.json"
}
configure 1
configure 2
sha256sum "$build"/compile_commands.[12].json
