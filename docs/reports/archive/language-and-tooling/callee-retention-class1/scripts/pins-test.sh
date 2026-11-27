#!/usr/bin/env bash
# pins-test.sh NAME -- CALLEE-RETENTION-CLASS1: the two contract pin tests the
# card updates deliberately (compiler/tests/contract_loader.rs and
# qcore_contract_corpus.rs), under the shared heavy lock, exits to files.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/crc1/pins-test-$name"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
for test in contract_loader qcore_contract_corpus; do
  flock "$lock" cargo test -p qcore-compiler --test "$test" > "$out/$test.log" 2>&1
  echo $? > "$out/$test.exit"
  echo "$test $(cat "$out/$test.exit")"
done
