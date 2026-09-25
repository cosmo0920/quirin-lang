#!/bin/sh
# The 47-module selection with the natural load-path spelling.
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab16a94e01ece061d
cd "$root" || exit 2
providers=""
for entry in $(cat private/identity/providers.txt); do
  providers="$providers --provider $entry"
done
# shellcheck disable=SC2086
private/run-current.sh natural 16 "$root/private/bin/quirin-audit" \
  --load-path "$root/providers/contract-load-path.json" $providers
echo "natural exit=$?"
