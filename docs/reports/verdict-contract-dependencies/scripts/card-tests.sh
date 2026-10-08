#!/usr/bin/env bash
# card-tests.sh LOG -- VERDICT-CONTRACT-DEPENDENCIES: the card's own test
# targets under the shared heavy lock (a quick check before the standing gate).
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock /tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock \
  cargo test -p qcore-c-gateway --test contract_dependencies \
  -p qcore-contract --test contract_premise \
  -p qcore-audit --test contract_dependencies_report > "$1" 2>&1
echo "tests exit $?"
flock /tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock \
  cargo test -p qcore-schemas --lib contract_consultation > "$1.schemas" 2>&1
echo "schemas exit $?"
