#!/usr/bin/env bash
# W46-ACCESS-PATH-CENSUS: the provider workspace's cargo test, run only to
# measure the matrix gate block's provider_tests (this card touches no
# providers/ path, so the provider gate is otherwise not required).
# usage: provider_count.sh LOG_DIR
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$1"
mkdir -p "$logs"
cd "$repo" || exit 2
export CARGO_INCREMENTAL=0
cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline > "$logs/provider-tests.log" 2>&1
echo $? > "$logs/provider-tests.exit"
cargo clean --manifest-path providers/Cargo.toml --profile dev > "$logs/provider-clean-dev.log" 2>&1
echo $? > "$logs/provider-clean-dev.exit"
