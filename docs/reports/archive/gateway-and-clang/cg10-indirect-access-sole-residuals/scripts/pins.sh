#!/usr/bin/env bash
# pins.sh NAME PACKAGE:TEST... -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: run
# named integration test binaries (package and test target) under the shared
# heavy lock; output and exits to private/cg10res/pins-NAME/, nothing piped.
set -u
name="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/cg10res/pins-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
for spec in "$@"; do
  package="${spec%%:*}"; test="${spec#*:}"
  flock "$lock" cargo test -p "$package" --test "$test" > "$out/$test.log" 2>&1
  echo $? > "$out/$test.exit"
  echo "$test $(cat "$out/$test.exit")"
done
