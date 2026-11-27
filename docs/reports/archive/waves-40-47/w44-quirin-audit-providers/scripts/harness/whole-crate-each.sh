#!/bin/sh
# Probe: each provider crate selected whole, alone. A run that reaches the
# (deliberately absent) compile database has loaded its contracts.
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab16a94e01ece061d
build=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/current-revision
cd "$root" || exit 2
for crate in qcore-provider-chunkio qcore-provider-cmetrics qcore-provider-ctraces \
  qcore-provider-libevent qcore-provider-librdkafka quirin-provider-cfl \
  quirin-provider-fluent-bit quirin-provider-libc quirin-provider-mpack \
  quirin-provider-msgpack quirin-provider-shared; do
  out=$("$root/private/bin/quirin-audit" current \
    --checkout "$build/fluent-bit-clang-source" \
    --profile-directory contract/profiles/fluent-bit \
    --compile-database /nonexistent/compile_commands.json \
    --load-path providers/contract-load-path.json \
    --provider "$crate" 2>&1)
  status=$?
  case "$out" in
    *"No such file or directory"*) verdict="loads" ;;
    *) verdict="refused: $(printf '%s' "$out" | tail -1 | cut -c1-400)" ;;
  esac
  echo "$crate exit=$status $verdict"
done
