#!/usr/bin/env bash
# census.sh NAME -- W43-ABI-ENUM-AND-CONST-PP: the production declared-ABI
# decision (`declared_abi_census`) for the card's three modules over the
# preprocessed member units (scripts/preprocess.py into private/rrk/units),
# into private/rrk/census-NAME.{log,exit}. A reading aid before the
# arbiter, which is the measurement. Under the shared heavy lock.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
flock "$lock" cargo run -q -p qcore-c-gateway --example declared_abi_census -- \
  providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv \
  providers/qcore-provider-librdkafka/contracts/librdkafka/lz4_stream.qv \
  providers/quirin-provider-libc/contracts/libc/process.qv -- \
  private/rrk/units/*.i > "private/rrk/census-$name.log" 2>&1
echo $? > "private/rrk/census-$name.exit"
echo "census $(cat "private/rrk/census-$name.exit")"
