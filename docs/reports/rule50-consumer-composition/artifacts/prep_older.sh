#!/bin/sh
# prep_older.sh: a Fluent Bit tree at 046accf6 for the pin tool (no link).
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a4697725f5e3891df
B=$W/private/base-src/target/release/quirin-audit
E=/home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
cd "$W" || exit 2
sh "$E" "$B" prepare clang-build \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit --revision 046accf6 \
  --output "$W/private/prep-older" --clang /usr/lib/llvm-21/bin/clang \
  --clangxx /usr/lib/llvm-21/bin/clang++ --repository "$W/private/base-src" \
  > "$W/private/logs/prep-older.log" 2>&1
e=$?
echo "exit $e" >> "$W/private/logs/prep-older.log"
exit $e
