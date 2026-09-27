#!/usr/bin/env bash
# W46-ACCESS-PATH-CENSUS: build the census-feature release audit binary into a
# separate target directory, so the default release binary the arbiter uses is
# untouched. usage: build_census.sh LOG_DIR
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$1"
mkdir -p "$logs"
cd "$repo" || exit 2
export CARGO_INCREMENTAL=0
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit \
  --features qcore-c-gateway/indirect-census --target-dir target/census \
  > "$logs/build-census.log" 2>&1
echo $? > "$logs/build-census.exit"
