#!/usr/bin/env bash
# targeted-tests.sh NAME -- RETENTION-UNMATCHED-REMOVAL-POISONS: under the
# shared heavy lock, the card's own test targets (the gateway's retention
# tests and unit tests, the contract loader's event-loop clauses, the contract
# corpus pins). Log and exit to private/rup/NAME.{log,exit}.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo" || exit 2
mkdir -p private/rup
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" /usr/bin/env bash -c '
  cargo test -p qcore-c-gateway --test event_loop_retention --test callee_retention_class1 \
    --test refcount_reference_returning --test retention_guard &&
  cargo test -p qcore-c-gateway --lib retained_by &&
  cargo test -p qcore-contract --test event_loop_clauses &&
  cargo test -p qcore-compiler --test qcore_contract_corpus
' > "private/rup/$name.log" 2>&1
echo $? > "private/rup/$name.exit"
cat "private/rup/$name.exit"
