#!/usr/bin/env bash
# targeted.sh NAME -- CALLEE-RETENTION-CLASS1: the card's own tests under the
# shared heavy lock (the gateway's unit tests of the two new modules, the
# gateway integration controls, the loader's clause tests, and the relational
# and pin tests the side tables touch), each exit to a file; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/crc1/targeted-$name"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  flock "$lock" "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
  echo "$label $(cat "$out/$label.exit")"
}
step gateway-lib cargo test -p qcore-c-gateway --lib -- retained_by relational_forwarding legacy_relational
step gateway-controls cargo test -p qcore-c-gateway --test callee_retention_class1
step gateway-relational cargo test -p qcore-c-gateway --test relational_postconditions
step contract-clause cargo test -p qcore-contract --test retained_by_clause
step contract-revision cargo test -p qcore-contract --test contract_revision_mode
step schemas cargo test -p qcore-schemas
