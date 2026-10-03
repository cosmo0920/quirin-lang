#!/usr/bin/env bash
# targeted.sh NAME TEST... -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: under the
# shared heavy lock, `cargo check` the gateway with its tests, then run each
# named gateway integration test binary; every output and exit to
# private/cg10res/targeted-NAME/, nothing piped.
set -u
name="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/cg10res/targeted-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo check -p qcore-c-gateway --all-targets > "$out/check.log" 2>&1
echo $? > "$out/check.exit"
[ "$(cat "$out/check.exit")" = 0 ] || { echo "check failed"; exit 1; }
for test in "$@"; do
  flock "$lock" cargo test -p qcore-c-gateway --test "$test" > "$out/$test.log" 2>&1
  echo $? > "$out/$test.exit"
  echo "$test $(cat "$out/$test.exit")"
done
