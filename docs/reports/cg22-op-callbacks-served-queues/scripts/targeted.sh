#!/usr/bin/env bash
# targeted.sh NAME [FILTER...] -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES: the
# card's own tests under the shared heavy lock (not the gate): the schema and
# loader units, the gateway's callback, closure and arm units, the clause
# standing and dependency-report bindings, and the card's integration test.
# Log and exit into private/cg22op/targeted-NAME.{log,exit}. Nothing piped.
set -u
name="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg22op"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
{
  flock "$lock" cargo test -p qcore-schemas --lib arm_release
  echo "schemas $?"
  flock "$lock" cargo test -p qcore-contract --test callback_clause
  echo "callback_clause $?"
  flock "$lock" cargo test -p qcore-contract --test op_callback_clause
  echo "op_callback_clause $?"
  flock "$lock" cargo test -p qcore-c-gateway --lib callback
  echo "gateway-callback $?"
  flock "$lock" cargo test -p qcore-c-gateway --lib arm_release_check
  echo "gateway-arm $?"
  flock "$lock" cargo test -p qcore-c-gateway --test contract_clause_standing
  echo "clause_standing $?"
  flock "$lock" cargo test -p qcore-c-gateway --test cg22_callback_registry
  echo "cg22_callback_registry $?"
  flock "$lock" cargo test -p qcore-c-gateway --test cg22_op_callbacks
  echo "cg22_op_callbacks $?"
  flock "$lock" cargo test -p qcore-audit --test contract_dependencies_report
  echo "dependencies_report $?"
} > "$out/targeted-$name.log" 2>&1
grep -E '^[a-z0-9_-]+ [0-9]+$' "$out/targeted-$name.log" > "$out/targeted-$name.exit"
cat "$out/targeted-$name.exit"
