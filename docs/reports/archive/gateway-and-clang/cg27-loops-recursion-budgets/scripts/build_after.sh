#!/usr/bin/env bash
# build_after.sh -- CG27-LOOPS-RECURSION-BUDGETS: build the AFTER release
# quirin-audit into a private target directory, so the BEFORE system and slice
# runs (which read target/release) keep the base binary while it builds. The
# AFTER is then measured with SKIP_BUILD=1 BINARY=<this binary> measure.sh.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
CARGO_TARGET_DIR="$repo/private/cg27lrb-target" CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 \
  cargo build --release -p qcore-audit --bin quirin-audit \
  > "$repo/private/cg27lrb/build-after.log" 2>&1
echo $? > "$repo/private/cg27lrb/build-after.exit"
