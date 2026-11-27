#!/usr/bin/env bash
# census.sh NAME -- SUPPLIED-OWNED-RETURN-CODE-HANDOFF: build and run the
# `handoff_census` example (crates/qcore-audit/examples/handoff_census.rs)
# over every librdkafka unit of the production compile database, as the
# pinned route's pre-pass reads them, under the shared heavy lock; output to
# private/sorch/handoff-census-NAME.{log,json,exit}.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/sorch"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
database=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/fluent-bit-clang-production/compile_commands.json
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo run -q --release --target-dir "$out/target-rel" -p qcore-audit \
  --example handoff_census -- "$database" librdkafka-2.15.0/src/ 6 \
  "$out/handoff-census-$name.json" > "$out/handoff-census-$name.log" 2>&1
echo $? > "$out/handoff-census-$name.exit"
