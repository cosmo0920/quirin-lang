#!/bin/sh
# usage: series.sh SUFFIX  -- the identity series on the binaries in private/bin
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab16a94e01ece061d
s=$1
profile="$root/compiler/../contract/profiles/fluent-bit"
cd "$root" || exit 2
providers=""
for entry in $(cat private/identity/providers.txt); do
  providers="$providers --provider $entry"
done
private/run-example.sh "example$s" 16 "$root/private/bin/example"; echo "example$s exit=$?"
private/run-current.sh "manifest$s" 16 "$root/private/bin/quirin-audit" \
  --contract-manifest "$profile/../../../cycle19-resource-contracts.json"; echo "manifest$s exit=$?"
# shellcheck disable=SC2086
private/run-current.sh "providers-cli$s" 16 "$root/private/bin/quirin-audit" \
  --load-path "$profile/../../../providers/contract-load-path.json" $providers; echo "providers-cli$s exit=$?"
private/run-current.sh "providers-config$s" 16 "$root/private/bin/quirin-audit" \
  --config "$root/private/identity/providers-config.json"; echo "providers-config$s exit=$?"
private/run-current.sh "contracts-only$s" 16 "$root/private/bin/quirin-audit" \
  --config "$root/private/identity/providers-config.json" --contracts-only; echo "contracts-only$s exit=$?"
echo "series$s done"
