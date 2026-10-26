#!/bin/sh
# CONTRACT-SUPERSESSION: the crates and pin-lock tests the card touches, before
# the scoped card gate (which runs the full suite). Usage: targeted_tests.sh LOG_DIR
set -u
logs=$1
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
mkdir -p "$logs"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
step() {
  name=$1
  shift
  "$@" >"$logs/$name.log" 2>&1
  echo $? >"$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
step schemas-contract cargo test -p qcore-schemas -p qcore-contract --no-fail-fast
step audit cargo test -p qcore-audit --lib --test contract_revision_audit \
  --test contract_revision_identities --test contract_tip_mode --test bounds_pin_hold \
  --test bounds_transport --test outer_projection --test contract_supersession --no-fail-fast
step gateway cargo test -p qcore-c-gateway --lib --test contract_pin_hold \
  --test contract_conflict_hold --no-fail-fast
