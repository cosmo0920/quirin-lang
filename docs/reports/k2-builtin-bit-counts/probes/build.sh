#!/bin/sh
# build.sh NAME TEST -- build one qcore-c-gateway test binary (no run) with the
# gate's test-link config; log and exit under private/k2bc/.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ae71e0b9f85fdedfd
cd "$W" || exit 2
name="$1"; shift
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo test \
  --config build.incremental=false \
  --config "build.rustc-wrapper=\"$W/scripts/rustc_test_dylib.py\"" \
  -p qcore-c-gateway "$@" --no-run > "$W/private/k2bc/$name.log" 2>&1
echo $? > "$W/private/k2bc/$name.exit"
cat "$W/private/k2bc/$name.exit"
grep -o 'Executable.*' "$W/private/k2bc/$name.log" | tail -3
