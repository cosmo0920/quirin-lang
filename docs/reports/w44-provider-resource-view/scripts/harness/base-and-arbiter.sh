#!/bin/sh
# usage: base-and-arbiter.sh SUFFIX BASE_REV
# 1. The base binary's manifest run. The base load-path reader refuses the new
#    `resource_view` field (deny_unknown_fields), so for this one run the load
#    path's bytes are rewritten to the base commit's, then rewritten back and
#    the round trip asserted by SHA-256 (no other run is in flight).
# 2. The pinned arbiter on the branch's release example, and its
#    verdict-bearing identity against the W44-COLLECTION-ALLOCATION baseline.
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-aa6cef7bd1d8bab9c
s=$1
base_rev=$2
h="$root/docs/reports/w44-provider-resource-view/scripts/harness"
profile="$root/compiler/../contract/profiles/fluent-bit"
cd "$root" || exit 2
lp=providers/contract-load-path.json
before=$(sha256sum "$lp" | cut -d' ' -f1)
cp "$lp" "private/load-path-branch.json"
git show "$base_rev:$lp" > "private/load-path-base.json"
cat private/load-path-base.json > "$lp"
"$h/run-current.sh" "base$s" 16 "$root/private/bin/quirin-audit-base" \
  --contract-manifest "$profile/../../../cycle19-resource-contracts.json"; echo "base$s exit=$?"
cat private/load-path-branch.json > "$lp"
touch "$lp"
after=$(sha256sum "$lp" | cut -d' ' -f1)
if [ "$before" != "$after" ]; then
  echo "load path round trip FAILED: $before -> $after"
  exit 3
fi
echo "load path restored: $after"
commit=$(git rev-parse HEAD)
python3 docs/reports/w43-return-expression/scripts/run_audit.py "private/arbiter$s" "$commit" --jobs 16 > "private/logs/arbiter$s.log" 2>&1
echo "arbiter$s exit=$?"
python3 docs/reports/w44-branch-condition-calls/scripts/baseline_identity.py "private/arbiter$s" \
  docs/reports/w44-collection-allocation/integration/artifacts \
  --root-alias "$root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da" \
  > "private/identity/baseline-identity-arbiter$s.json" 2>&1
echo "baseline_identity$s exit=$?"
