#!/usr/bin/env bash
# run_repro.sh BINARY CHECKOUT_OF_TIP OUT_DIR [monkey]: unit-scoped `current`
# (K5's run_repro.sh, Monkey optional) on the filtered compile database
# private/repair/compile_commands.json. Inputs read in place; output under OUT_DIR.
set -euo pipefail
binary=$1
tip=$2
out=$3
here=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a86a9bde776af1664
prepared=/media/Data/GitHub/quirin-lang/build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source
mkdir -p "$out"
extra=()
if [ "${4:-}" = monkey ]; then
  extra=(--monkey-database "$here/private/repair/monkey_compile_commands.json"
         --monkey-linked --monkey-source-report "$out/monkey-source.json")
fi
export QCORE_CLANG=/usr/lib/llvm-21/bin/clang
cd "$tip"
exec "$binary" current \
  --checkout "$prepared/fluent-bit-clang-source" \
  --profile-directory "$tip/contract/profiles/fluent-bit" \
  --compile-database "$here/private/repair/compile_commands.json" \
  "${extra[@]}" \
  --report "$out/resource-diagnostics.json" \
  --jobs 3 >"$out/console.log" 2>&1
