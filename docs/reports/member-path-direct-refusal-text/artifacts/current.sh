#!/bin/sh
# current.sh SIDE OUT: quirin-audit current with SIDE's binary over the
# 66910c10 preparation, the Monkey standalone units plus one chunkio file.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aa4af16c3e0cb085f
S=$1 OUT=$2
P=$W/private/prep-66910c10
mkdir -p "$W/measure"
mkdir "$OUT" || exit 2
cd "$OUT" || exit 2
SOURCE_DATE_EPOCH=1767225600 sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh \
  "$W/private/$S-src/target/release/quirin-audit" current \
  --checkout "$P/fluent-bit-clang-source" \
  --profile-directory "$W/private/$S-src/contract/profiles/fluent-bit" \
  --compile-database "$P/fluent-bit-clang-production/compile_commands.json" \
  --monkey-database "$P/monkey/monkey-build/compile_commands.json" \
  --selection "$W/private/monkey-only-selection.json" \
  --report "$OUT/resource-diagnostics.json" \
  --monkey-source-report "$OUT/monkey-source.json" \
  --jobs 8 > "$OUT/run.log" 2>&1
echo $? > "$OUT/exit"
cat "$OUT/exit"
