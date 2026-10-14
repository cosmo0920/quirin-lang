#!/usr/bin/env bash
# targeted.sh NAME TEST... -- OP-CALLBACK-CANDIDATE-CONTRACTS: under the shared
# heavy lock, `cargo test -p qcore-compiler` for the named integration tests
# (the contract pin tables), output and exit to private/opcand/targeted-NAME.*.
set -u
name="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/opcand"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
args=()
for test in "$@"; do args+=(--test "$test"); done
flock "$lock" cargo test -p qcore-compiler "${args[@]}" > "$out/targeted-$name.log" 2>&1
echo $? > "$out/targeted-$name.exit"
echo "targeted $name $(cat "$out/targeted-$name.exit")"
