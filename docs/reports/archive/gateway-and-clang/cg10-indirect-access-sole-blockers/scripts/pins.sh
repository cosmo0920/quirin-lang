#!/usr/bin/env bash
# pins.sh NAME -- CG10-INDIRECT-ACCESS-SOLE-BLOCKERS: the suites whose pins the
# card flips deliberately (neutral_expression_positions,
# summary_validation_premises) and the card's own, under the shared heavy lock,
# each to its own log and exit under private/cg10sole/pins-NAME. Nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg10sole/pins-$name"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
for suite in neutral_expression_positions summary_validation_premises indirect_access_sole_blockers; do
  flock "$lock" cargo test -p qcore-c-gateway --test "$suite" > "$out/$suite.log" 2>&1
  echo $? > "$out/$suite.exit"
done
for f in "$out"/*.exit; do echo "$(basename "$f" .exit) $(cat "$f")"; done
