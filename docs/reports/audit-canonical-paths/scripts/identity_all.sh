#!/bin/sh
# Every attempt-2 comparison: bytes (compare_runs2.sh), verdicts
# (verdicts_all.sh) and the clock-dependent key (time_key_all.sh), per route.
# Usage: identity_all.sh WORKTREE [NAMESPACE_CORPUS_RUNS]
W=$1
S=$W/docs/reports/audit-canonical-paths/scripts
for route in corpus slice system; do
  sh "$S/compare_runs2.sh" "$W" "$route"
  sh "$S/verdicts_all.sh" "$W" "$route"
  sh "$S/time_key_all.sh" "$W" "$route"
done
