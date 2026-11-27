#!/usr/bin/env bash
# remeasure.sh BEFORE-COMMIT AFTER-BRANCH -- CAST-FORMALIZATION, after the
# rebase onto main 9c5f2079c: build the census instrument commit (main plus
# the verdict-neutral census hook) and measure it as `before2`, then build the
# card's tip and measure it as `after2`, then the tip probe. Each build and
# route runs under the shared heavy lock (build.sh, measure.sh). The worktree
# is checked out at BEFORE-COMMIT only while that build runs.
set -u
before="$1"
branch="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
git checkout -q "$before" || exit 2
"$here/build.sh" before2 cast_semantics
git checkout -q "$branch" || exit 2
"$here/measure.sh" before2
"$here/build.sh" after2 cast_semantics
"$here/measure.sh" after2
"$here/tip-probe.sh" after2
echo done > "$repo/private/castf/remeasure-done"
