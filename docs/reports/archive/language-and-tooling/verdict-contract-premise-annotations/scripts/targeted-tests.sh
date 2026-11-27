#!/usr/bin/env bash
# targeted-tests.sh NAME -- VERDICT-CONTRACT-PREMISE-ANNOTATIONS: the contract
# corpus pins and the loader tests, under the shared heavy lock, before the
# full gate. Exit to private/vcpa/NAME.exit, log beside it; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo test -p qcore-compiler --test qcore_contract_corpus --test contract_loader \
  > "private/vcpa/$name.log" 2>&1
echo $? > "private/vcpa/$name.exit"
cat "private/vcpa/$name.exit"
