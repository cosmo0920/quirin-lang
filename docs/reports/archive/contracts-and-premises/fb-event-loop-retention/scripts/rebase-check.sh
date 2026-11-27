#!/usr/bin/env bash
# rebase-check.sh NAME -- FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: the
# targeted check after a rebase onto a main that moved with disjoint gated
# commits (landing-race policy): cargo check, the conflict-marker grep, the
# route tests of both cards, the Lean audits, the docs index check and the
# bold rule-id duplicate check, each under the shared heavy lock, every exit
# to private/fbel/rebase-NAME/.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/fbel/rebase-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  flock "$lock" "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
git rev-parse HEAD > "$out/head"
git rev-parse main > "$out/main"
step 00-check cargo check --workspace --all-targets
git grep -n '^<<<<<<<\|^>>>>>>>' > "$out/01-markers.log" 2>&1
echo $? > "$out/01-markers.exit"
step 02-route-tests cargo test -p qcore-c-gateway --test event_loop_retention --test return_code_handoff --test handoff_direct_call --test callee_retention_class1 --test refcount_reference_returning
step 03-loader-tests cargo test -p qcore-contract --test event_loop_clauses
step 04-lean python3 scripts/check_lean_audits.py
step 05-index python3 scripts/build_docs_index.py --check
rg -o --no-filename '^[0-9]{1,2}[a-z]?\. \*\*' docs/agents/ > "$out/rule-ids.txt" 2>&1
sort "$out/rule-ids.txt" | uniq -d > "$out/06-rule-ids.dup"
echo $? > "$out/06-rule-ids.exit"
grep -H . "$out"/*.exit
wc -l < "$out/06-rule-ids.dup"
