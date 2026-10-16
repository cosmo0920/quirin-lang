#!/usr/bin/env bash
# arbiter.sh MAIN TIPNAME MAINNAME -- FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE: the combined
# arbiter after a rebase. Detach the worktree at MAIN, build main's release
# binary and measure it on the corpus, system and slice routes with the null
# property, all on main's tree (contracts are read from the tree at run time),
# then return to the card branch, build the tip's binary and measure it.
# Builds and runs use copies of build.sh and measure.sh under private/fbn/scripts
# (repository root two levels up), since main's tree has no copy of them;
# every heavy step takes the shared lock inside those scripts. Afterwards the
# release target is deleted (the disk rule). Run a copy of this script from
# outside the tree (it checks other revisions out under itself); the worktree
# path is this card's.
set -u
main="$1"; tip="$2"; mainname="$3"
repo=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a0065d704c0c6451e
cd "$repo" || exit 2
here="$repo/docs/reports/fb-plugin-descriptor-nonnull/scripts"
branch=worktree-agent-a0065d704c0c6451e
mkdir -p private/fbn/scripts
cp "$here/build.sh" "$here/measure.sh" private/fbn/scripts/
sed -i 's#repo="$(cd "$here/../../../.." \&\& pwd)"#repo="$(cd "$here/../../.." \&\& pwd)"#' private/fbn/scripts/build.sh private/fbn/scripts/measure.sh
git checkout -q --detach "$main" || exit 3
git submodule update --init --recursive > private/fbn/arbiter-$mainname-submodule.log 2>&1
bash private/fbn/scripts/build.sh "$mainname" || { git checkout -q "$branch"; exit 4; }
bash private/fbn/scripts/measure.sh "$mainname" "private/fbn/bin/$mainname" corpus,system,slice
git checkout -q "$branch" || exit 5
git submodule update --init --recursive > private/fbn/arbiter-$tip-submodule.log 2>&1
bash private/fbn/scripts/build.sh "$tip" || exit 6
bash private/fbn/scripts/measure.sh "$tip" "private/fbn/bin/$tip" corpus,system,slice
rm -rf private/fbn/target-rel
echo done > "private/fbn/arbiter-$tip.done"
