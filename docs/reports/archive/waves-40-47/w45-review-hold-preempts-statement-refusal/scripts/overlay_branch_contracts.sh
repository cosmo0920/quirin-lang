#!/usr/bin/env bash
# overlay_branch_contracts.sh PROBE_TREE -- write W47-LOOP-OWNER-INSTANCE-FLOW's
# four contract-data files (read from its branch, never merged) into
# PROBE_TREE, an export of this card's tip outside qcore-c/. Run from this
# worktree. Exits nonzero if any read fails.
set -eu
tree="$1"
branch=worktree-agent-aa0821f6bf47bd9c0
for path in \
  providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_instance.qv \
  providers/quirin-provider-fluent-bit/contracts.lock \
  providers/contract-load-path.json \
  cycle19-resource-contracts.json; do
  git show "$branch:$path" > "$tree/$path"
done
