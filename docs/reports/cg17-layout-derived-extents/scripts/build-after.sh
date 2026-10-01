#!/usr/bin/env bash
# build-after.sh -- CG17-LAYOUT-DERIVED-EXTENTS: build the AFTER release
# quirin-audit into a separate target directory under private/cg17le, so a
# BEFORE run still reading target/release is never handed a new binary.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg17le"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
CARGO_TARGET_DIR="$out/target-after" CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 \
  cargo build --release -p qcore-audit --bin quirin-audit > "$out/build-after.log" 2>&1
echo $? > "$out/build-after.exit"
