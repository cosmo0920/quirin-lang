#!/usr/bin/env bash
# after-chain.sh NAME -- HANDOFF-DIRECT-CALL-READING: build the AFTER binary
# NAME from the checked-out tree (build.sh), run the census example over the
# librdkafka units (census.sh), then measure NAME on every route with the null
# property (after.sh). Each step takes the shared heavy lock itself; each
# step's exit is written to private/hdcr/NAME-chain.*.exit. Nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/hdcr"
mkdir -p "$out" || exit 2
"$here/build.sh" "$name" > "$out/$name-build.out" 2>&1
echo $? > "$out/$name-chain.build.exit"
[ "$(cat "$out/$name-chain.build.exit")" = 0 ] || exit 1
"$here/census.sh" "$name"
echo $? > "$out/$name-chain.census.exit"
"$here/after.sh" "$name" "$out/bin/$name"
echo $? > "$out/$name-chain.after.exit"
