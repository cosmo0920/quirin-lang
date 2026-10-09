#!/usr/bin/env bash
# controls.sh NAME -- RDHTTP-HEADER-SUMMARY-GAP: run the card's control tests
# (crates/qcore-c-gateway/tests/deep_unit_ast.rs) under the shared heavy lock
# into private/rhsg/controls-NAME.{log,exit}, then the module's unit tests
# (deep_unit_ast::tests) into private/rhsg/unit-NAME.{log,exit}. Nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$repo/private/rhsg" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo test -p qcore-c-gateway --test deep_unit_ast > "$repo/private/rhsg/controls-$name.log" 2>&1
echo $? > "$repo/private/rhsg/controls-$name.exit"
cat "$repo/private/rhsg/controls-$name.exit"
flock "$lock" cargo test -p qcore-c-gateway --lib deep_unit_ast > "$repo/private/rhsg/unit-$name.log" 2>&1
echo $? > "$repo/private/rhsg/unit-$name.exit"
cat "$repo/private/rhsg/unit-$name.exit"
