#!/usr/bin/env bash
# identity.sh RUN_ROOT SLICE_DIR OUT_DIR: each measurement of a run against the
# W47-LOOP-OWNER-CONTRACTS baseline archive (filename-keyed manifests, built in
# the worktree agent-a04a30eb19f1c410e at 9b3f9f88), with this checkout's root
# aliased to the baseline's, plus compose3 (verdicts, leaves, counts).
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
run="$1"; slice="$2"; out="$3"
baseline_root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a04a30eb19f1c410e
archive="$repo/docs/reports/w47-loop-owner-contracts/artifacts/after"
compose=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-implementation-wave-47-681db5/c8ef992e-a75f-4250-b4e1-07e6e5477601/scratchpad/compose3.py
mkdir -p "$out"
for measure in corpus system slice; do
  dir="$run/$measure"
  [ "$measure" = slice ] && dir="$slice"
  python3 "$here/manifest_identity.py" "$dir" "$archive/$measure" \
    --root-alias "$repo=$baseline_root" > "$out/$measure.identity" 2>&1
  echo "$measure identity $?"
  python3 "$compose" "$archive/$measure" "$archive/$measure" "$archive/$measure" "$dir" \
    > "$out/$measure.compose" 2>&1
  echo "$measure compose $? $(tail -n 1 "$out/$measure.compose")"
done
