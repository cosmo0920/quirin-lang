#!/usr/bin/env bash
# parity_all.sh OUT_DIR -- the corpus parity stream of card
# W47-REVIEW-HOLD-CONCRETE-REASON as a multiset: BEFORE against the baseline
# archive (run roots aliased) and AFTER against BEFORE. (measure.sh records
# the stream on the corpus route only.) Each exit is written to a file;
# nothing is piped.
set -u
out="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
base=$repo/docs/reports/w47-loop-owner-instance-flow/integration/artifacts
base_root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/implementation-wave-47-681db5
mkdir -p "$out"
for scope in corpus; do
  before_run="$repo/private/cr/before/corpus/parity.jsonl"
  after_run="$repo/private/cr/after/corpus/parity.jsonl"
  python3 "$here/parity_multiset.py" "$base/$scope/parity.jsonl" "$before_run" \
    --alias "$repo=$base_root" > "$out/$scope-baseline-before.log" 2>&1
  echo $? > "$out/$scope-baseline-before.exit"
  python3 "$here/parity_multiset.py" "$before_run" "$after_run" \
    > "$out/$scope-before-after.log" 2>&1
  echo $? > "$out/$scope-before-after.exit"
done
