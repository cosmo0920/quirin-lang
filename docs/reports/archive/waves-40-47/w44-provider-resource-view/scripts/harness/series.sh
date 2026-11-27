#!/bin/sh
# usage: series.sh SUFFIX -- the identity series on the binaries in private/bin.
#   manifest     the Cycle 19 manifest (full mode), branch binary
#   (the base binary's run is base-and-arbiter.sh's: it needs the base load path)
#   whole        all eleven crates named whole (full mode)
#   canonical    a hand-written manifest: the Cycle 19 manifest's 47 entries in
#                the order a whole-crate selection lowers them, beside it
#   all-loaded   --contracts-only with no selection (the all-loaded default)
#   canonical-co the canonical manifest with --contracts-only
#   manifest-co  the Cycle 19 manifest with --contracts-only
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-aa6cef7bd1d8bab9c
s=$1
h="$root/docs/reports/w44-provider-resource-view/scripts/harness"
profile="$root/compiler/../contract/profiles/fluent-bit"
load_path="$profile/../../../providers/contract-load-path.json"
cd "$root" || exit 2
crates=""
for name in $(python3 -c "
import json
print(' '.join(c['name'] for c in json.load(open('providers/contract-load-path.json'))['crates']))"); do
  crates="$crates --provider $name"
done
bin="$root/private/bin/quirin-audit"
"$h/run-current.sh" "manifest$s" 16 "$bin" \
  --contract-manifest "$profile/../../../cycle19-resource-contracts.json"; echo "manifest$s exit=$?"
# shellcheck disable=SC2086
"$h/run-current.sh" "whole$s" 16 "$bin" --load-path "$load_path" $crates; echo "whole$s exit=$?"
"$h/run-current.sh" "canonical$s" 16 "$bin" \
  --contract-manifest "$profile/../../../cycle19-canonical-order.json"; echo "canonical$s exit=$?"
"$h/run-current.sh" "all-loaded$s" 16 "$bin" --load-path "$load_path" --contracts-only
echo "all-loaded$s exit=$?"
"$h/run-current.sh" "canonical-co$s" 16 "$bin" \
  --contract-manifest "$profile/../../../cycle19-canonical-order.json" --contracts-only
echo "canonical-co$s exit=$?"
"$h/run-current.sh" "manifest-co$s" 16 "$bin" \
  --contract-manifest "$profile/../../../cycle19-resource-contracts.json" --contracts-only
echo "manifest-co$s exit=$?"
echo "series$s done"
