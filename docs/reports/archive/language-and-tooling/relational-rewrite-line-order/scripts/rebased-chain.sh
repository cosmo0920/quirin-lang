#!/usr/bin/env bash
# rebased-chain.sh MAIN-REV MAIN-NAME TIP-NAME -- RELATIONAL-REWRITE-LINE-ORDER
# (adapted from HANDOFF-DIRECT-CALL-READING): after the rebase, the targeted
# check (cargo check --workspace --all-targets under the heavy lock), main's
# binary from an export of MAIN-REV (build-main.sh) and the rebased tip's
# (build.sh), each measured on every route with the null property
# (measure-binary.sh), then the census of the tip. Exits to private/rrlo/*.exit;
# nothing piped.
set -u
rev="$1"; main="$2"; tip="$3"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/rrlo"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo" || exit 2
git rev-parse HEAD > "$out/rebased-check.head"
CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 flock "$lock" cargo check --workspace --all-targets > "$out/rebased-check.log" 2>&1
echo $? > "$out/rebased-check.exit"
[ "$(cat "$out/rebased-check.exit")" = 0 ] || exit 1
"$here/build-main.sh" "$rev" "$main" > "$out/$main-build.out" 2>&1
echo $? > "$out/$main-chain.build.exit"
[ "$(cat "$out/$main-chain.build.exit")" = 0 ] || exit 1
NULL=1 "$here/measure-binary.sh" "$main" "$out/bin/$main" > "$out/$main-measure.out" 2>&1
echo $? > "$out/$main-chain.measure.exit"
"$here/build.sh" "$tip" > "$out/$tip-build.out" 2>&1
echo $? > "$out/$tip-chain.build.exit"
[ "$(cat "$out/$tip-chain.build.exit")" = 0 ] || exit 1
NULL=1 "$here/measure-binary.sh" "$tip" "$out/bin/$tip" > "$out/$tip-measure.out" 2>&1
echo $? > "$out/$tip-chain.measure.exit"
"$here/census.sh" "$tip" > "$out/$tip-census.out" 2>&1
echo $? > "$out/$tip-chain.census.exit"
