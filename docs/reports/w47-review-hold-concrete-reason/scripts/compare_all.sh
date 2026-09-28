#!/usr/bin/env bash
# compare_all.sh BASE_KIND RUN_PHASE OUT_DIR -- run compare.py on the three
# scopes of card W47-REVIEW-HOLD-CONCRETE-REASON.
#   BASE_KIND: `baseline` (the integration archive of
#              docs/reports/w47-loop-owner-instance-flow/integration/artifacts)
#              or a phase name measured by measure.sh (e.g. `before`).
#   RUN_PHASE: a phase measured by measure.sh (`before`, `after`, ...).
# Run roots are aliased to the baseline's worktree root for the baseline
# comparison. Each exit is written to OUT_DIR/<scope>.exit; nothing is piped.
set -u
kind="$1"; phase="$2"; out="$3"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
tail=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
mkdir -p "$out"
alias_args=()
if [[ "$kind" == baseline ]]; then
  base_root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/implementation-wave-47-681db5
  alias_args=(--alias "$repo=$base_root")
fi
for scope in corpus system slice; do
  if [[ "$kind" == baseline ]]; then
    base="$repo/docs/reports/w47-loop-owner-instance-flow/integration/artifacts/$scope"
  elif [[ "$scope" == slice ]]; then
    base="$tail/cr-$kind"
  else
    base="$repo/private/cr/$kind/$scope"
  fi
  if [[ "$scope" == slice ]]; then
    run="$tail/cr-$phase"
  else
    run="$repo/private/cr/$phase/$scope"
  fi
  python3 "$here/compare.py" "$base" "$run" "${alias_args[@]}" --json "$out/$scope.json" \
    > "$out/$scope.log" 2>&1
  echo $? > "$out/$scope.exit"
done
