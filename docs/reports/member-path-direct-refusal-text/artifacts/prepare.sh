#!/bin/sh
# Prepare the Fluent Bit 66910c10 inputs (production compile database and the
# standalone Libevent Monkey database) with the tip export's quirin-audit.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aa4af16c3e0cb085f
cd "$W" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh \
  "$W/private/tip-src/target/release/quirin-audit" prepare \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit --revision 66910c10 \
  --output "$W/private/prep-66910c10" \
  --clang /usr/lib/llvm-21/bin/clang --clangxx /usr/lib/llvm-21/bin/clang++ \
  --repository "$W/private/tip-src" > "$W/private/prep-66910c10.log" 2>&1
echo $? > "$W/private/prep-66910c10.exit"
cat "$W/private/prep-66910c10.exit"
