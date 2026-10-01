#!/usr/bin/env bash
# build_census.sh -- build the census-only quirin-audit (legacy_indirect_calls.rs
# with ADMIT = false: parity rows, no rewrite) into a private target directory,
# so the BEFORE run's binary in target/release is not replaced while it runs.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
CARGO_TARGET_DIR="$repo/private/cg22-target" CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 \
  cargo build --release -p qcore-audit --bin quirin-audit > "$repo/private/cg22/build-census.log" 2>&1
echo $? > "$repo/private/cg22/build-census.exit"
