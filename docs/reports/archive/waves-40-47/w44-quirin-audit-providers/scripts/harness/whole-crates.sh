#!/bin/sh
# Probe: the Cycle 19 crates selected whole (every owned module), contracts
# only; the compile database is deliberately absent, so the run stops after
# the contract load (which happens first) or at the database read.
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab16a94e01ece061d
build=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/current-revision
cd "$root" || exit 2
"$root/private/bin/quirin-audit" current \
  --checkout "$build/fluent-bit-clang-source" \
  --profile-directory contract/profiles/fluent-bit \
  --compile-database /nonexistent/compile_commands.json \
  --load-path providers/contract-load-path.json \
  --provider quirin-provider-libc --provider quirin-provider-cfl \
  --provider qcore-provider-cmetrics --provider qcore-provider-ctraces \
  --provider qcore-provider-libevent --provider quirin-provider-msgpack \
  --provider quirin-provider-mpack --provider qcore-provider-librdkafka \
  --provider quirin-provider-fluent-bit --provider qcore-provider-chunkio
echo "exit=$?"
