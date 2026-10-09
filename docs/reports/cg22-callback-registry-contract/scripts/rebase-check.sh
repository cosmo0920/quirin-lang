#!/usr/bin/env bash
# rebase-check.sh NAME BASE PRE_REBASE_TIP -- CG22-CALLBACK-REGISTRY-CONTRACT:
# the checks the round-21 brief asks after a rebase: `cargo check --workspace
# --all-targets` (under the shared lock), a conflict-marker grep, the
# scratchpad's audit_card_lines.sh BASE PRE_REBASE_TIP, and
# `providers/contracts.py check`. Outputs and exits to
# private/cg22r/rebase-NAME.*; nothing piped.
set -u
name="$1"; base="$2"; tip="$3"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
scratch=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad
out="$repo/private/cg22r/rebase-$name"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$scratch/heavy.lock" cargo check --workspace --all-targets > "$out.check.log" 2>&1
echo $? > "$out.check.exit"
git grep -n '^<<<<<<<\|^>>>>>>>' > "$out.markers.log" 2>&1
echo $? > "$out.markers.exit"
bash "$scratch/audit_card_lines.sh" "$base" "$tip" > "$out.audit.log" 2>&1
echo $? > "$out.audit.exit"
python3 providers/contracts.py check > "$out.contracts.log" 2>&1
echo $? > "$out.contracts.exit"
for step in check markers audit contracts; do
  echo "$step $(cat "$out.$step.exit") $(wc -l < "$out.$step.log") lines"
done
