#!/usr/bin/env bash
# check.sh NAME -- cargo check --workspace --all-targets after a rebase; log and
# exit under private/cg26va/check-NAME.*
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg26va"
cd "$repo" || exit 2
CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 cargo check --workspace --all-targets > "$out/check-$1.log" 2>&1
echo $? > "$out/check-$1.exit"
cat "$out/check-$1.exit"
