#!/usr/bin/env bash
# build-after.sh -- build the AFTER release quirin-audit into a separate target
# directory (private/cg26va/target-after) while the BEFORE system and slice
# runs still use target/release. Exit in private/cg26va/build-after.exit.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg26va"
cd "$repo" || exit 2
CARGO_TARGET_DIR="$out/target-after" CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 \
  cargo build --release -p qcore-audit --bin quirin-audit > "$out/build-after.log" 2>&1
echo $? > "$out/build-after.exit"
cat "$out/build-after.exit"
