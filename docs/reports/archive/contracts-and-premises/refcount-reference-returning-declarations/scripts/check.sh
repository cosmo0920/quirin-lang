#!/usr/bin/env bash
# check.sh NAME [TEST...] -- REFCOUNT-REFERENCE-RETURNING-DECLARATIONS: under
# the shared heavy lock, `cargo check --workspace --all-targets`, then (when
# TEST names are given) `cargo test -p qcore-c-gateway --test TEST` for each
# and the gateway crate's library unit tests filtered to `reference` and
# `retained`. Output and exits to private/rrrd/check-NAME.*; nothing piped.
set -u
name="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/rrrd"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo check --workspace --all-targets > "$out/check-$name.log" 2>&1
echo $? > "$out/check-$name.exit"
echo "check $(cat "$out/check-$name.exit")"
[ "$(cat "$out/check-$name.exit")" = 0 ] || exit 1
if [ "$#" -gt 0 ]; then
  flock "$lock" cargo test -p qcore-c-gateway --lib -- reference retained > "$out/check-$name-lib.log" 2>&1
  echo $? > "$out/check-$name-lib.exit"
  echo "lib $(cat "$out/check-$name-lib.exit")"
  for test in "$@"; do
    case "$test" in
      contract:*) flock "$lock" cargo test -p qcore-contract --test "${test#contract:}" > "$out/check-$name-${test#contract:}.log" 2>&1 ;;
      compiler:*) flock "$lock" cargo test -p qcore-compiler --test "${test#compiler:}" > "$out/check-$name-${test#compiler:}.log" 2>&1 ;;
      *) flock "$lock" cargo test -p qcore-c-gateway --test "$test" > "$out/check-$name-$test.log" 2>&1 ;;
    esac
    echo $? > "$out/check-$name-${test#*:}.exit"
    echo "$test $(cat "$out/check-$name-${test#*:}.exit")"
  done
fi
