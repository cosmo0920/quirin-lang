#!/bin/sh
# release_tests.sh SOURCE LOG: the crate's ignored tests over the pinned
# release (SOURCE: the unpacked curl-8.18.0 `libcurl prepare` writes).
# Exit to LOG.exit; nothing is piped.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a717a6f321d7ff5d9
cd "$W" || exit 2
QUIRIN_LIBCURL_SOURCE=$1 cargo test -p quirin-audit-plugin-libcurl --test plugin -- --ignored \
  > "$2" 2>&1
echo $? > "$2.exit"
