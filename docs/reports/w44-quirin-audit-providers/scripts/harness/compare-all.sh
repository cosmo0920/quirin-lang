#!/bin/sh
# usage: compare-all.sh SUFFIX -- compare one identity series with the base run.
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab16a94e01ece061d
cd "$root" || exit 2
s=$1
c=docs/reports/w44-quirin-audit-providers/scripts/compare_outputs.py
o=private/identity
python3 "$c" private/runs/base "private/runs/example$s" > "$o/compare-base-vs-example$s.json"
echo "example$s exit=$?"
python3 "$c" private/runs/base "private/runs/manifest$s" --log-mask-lines 1,2 > "$o/compare-base-vs-manifest$s.json"
echo "manifest$s exit=$?"
for r in providers-cli providers-config; do
  python3 "$c" private/runs/base "private/runs/$r$s" --added resource-diagnostics.contracts.json:provider_selection \
    --log-mask-lines 1,2 > "$o/compare-base-vs-$r$s.json"
  echo "$r$s exit=$?"
done
python3 "$c" private/runs/base "private/runs/contracts-only$s" \
  --added resource-diagnostics.contracts.json:provider_selection \
  --added resource-diagnostics.contracts.json:evidence \
  --added resource-diagnostics.json:evidence \
  --log-mask-lines 1,2 --log-inserted 3 > "$o/compare-base-vs-contracts-only$s.json"
echo "contracts-only$s exit=$?"
python3 "$c" "private/runs/manifest$s" "private/runs/contracts-only$s" \
  --added resource-diagnostics.contracts.json:provider_selection \
  --added resource-diagnostics.contracts.json:evidence \
  --added resource-diagnostics.json:evidence \
  --log-inserted 3 > "$o/compare-manifest$s-vs-contracts-only$s.json"
echo "manifest$s-vs-contracts-only$s exit=$?"
