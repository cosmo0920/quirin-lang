#!/usr/bin/env bash
# arbiter.sh MAIN MAINNAME TIPNAME -- OUTER-CONTRACT (from FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE's
# arbiter.sh): the combined arbiter after a rebase. Detach this worktree at
# MAIN, build main's release binary and measure it on the corpus, system and
# slice routes with the null property (main ignores QCORE_OUTER_PROJECTION),
# all on main's tree; then return to the card branch, build the tip's binary
# and measure it with the projection. Builds and runs use copies of build.sh
# and measure.sh under private/oc/scripts, since main's tree has no copy of
# them; every heavy step takes the shared lock inside those scripts. The
# release targets are deleted afterwards (the disk rule). Run a copy of this
# script from outside the tree.
set -u
main="$1"; mainname="$2"; tip="$3"
repo=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-abed64dfe3188dfef
branch=worktree-agent-abed64dfe3188dfef
cd "$repo" || exit 2
here="$repo/docs/reports/outer-contract/scripts"
mkdir -p private/oc/scripts
cp "$here/build.sh" "$here/measure.sh" private/oc/scripts/
sed -i 's#repo="$(cd "$here/../../../.." \&\& pwd)"#repo="$(cd "$here/../../.." \&\& pwd)"#' private/oc/scripts/build.sh private/oc/scripts/measure.sh
git checkout -q --detach "$main" || exit 3
git submodule update --init --recursive > private/oc/arbiter-$mainname-submodule.log 2>&1
bash private/oc/scripts/build.sh "$mainname" || { git checkout -q "$branch"; exit 4; }
bash private/oc/scripts/measure.sh "$mainname" "private/oc/bin/$mainname" corpus,system,slice
git checkout -q "$branch" || exit 5
git submodule update --init --recursive > private/oc/arbiter-$tip-submodule.log 2>&1
bash private/oc/scripts/build.sh "$tip" || exit 6
bash private/oc/scripts/measure.sh "$tip" "private/oc/bin/$tip" corpus,system,slice
rm -rf private/oc/target-rel target/release/quirin-audit
echo done > "private/oc/arbiter-$tip.done"
