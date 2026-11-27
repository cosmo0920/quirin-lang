#!/usr/bin/env bash
# run-all.sh BASE BRANCH -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: build
# BEFORE `before2` with the worktree detached at BASE and AFTER `a2` back on
# BRANCH (build.sh, the worktree's own tree, so the compiled-in paths and the
# libevent submodule's git identity are the worktree's), then measure each on
# every route with the null property (after.sh). Nothing in the tree may be
# edited while it runs. Exits to private/mpuh/*.exit.
#
# The first attempt (`before` / `a1`, built from `git archive` exports by
# build-export.sh) measured the system and slice routes, but its corpus run
# failed in the Fluent Bit audit (`Pin(GitUnavailable { pin: "libevent" })`:
# an export carries no git identity for the submodule); it is kept as a failed
# attempt and not used as evidence.
set -u
base="$1"; branch="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/mpuh"
cp "$here/build.sh" "$here/after.sh" "$here/measure-binary.sh" "$here/measure.sh" "$out/" || exit 2
cd "$repo" || exit 2
git checkout -q --detach "$base" || exit 2
REPO="$repo" bash "$out/build.sh" before2 > "$out/run-build-before2.out" 2>&1
echo $? > "$out/run-build-before2.exit"
git checkout -q "$branch" || exit 2
REPO="$repo" bash "$out/build.sh" a2 > "$out/run-build-a2.out" 2>&1
echo $? > "$out/run-build-a2.exit"
[ "$(cat "$out/run-build-before2.exit")" = 0 ] && REPO="$repo" bash "$here/after.sh" before2 "$out/bin/before2"
[ "$(cat "$out/run-build-a2.exit")" = 0 ] && REPO="$repo" bash "$here/after.sh" a2 "$out/bin/a2"
echo done > "$out/run-all2.done"
