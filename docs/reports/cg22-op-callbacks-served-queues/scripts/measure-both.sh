#!/usr/bin/env bash
# measure-both.sh AFTER_NAME BEFORE_NAME -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES:
# (1) the AFTER: this branch's saved binary AFTER_NAME on every route with the
# null property, reading this branch's contracts; (2) the BEFORE: check out
# local `main` detached in this worktree, build BEFORE_NAME from it and measure
# it the same way, so the contracts it reads are main's; then check this
# branch out again. The scripts run from a copy in the scratchpad (main has
# none of them). Every exit goes to a file under private/cg22op/.
set -u
after="$1"; before="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
branch="$(git -C "$repo" rev-parse --abbrev-ref HEAD)"
copy=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/op-scripts
rm -rf "$copy" && cp -r "$here" "$copy" || exit 2
export REPO="$repo" NULL=1
"$copy/measure-binary.sh" "$after" "$repo/private/cg22op/bin/$after"
echo "after done"
[ -z "$(git -C "$repo" status --porcelain --untracked-files=no)" ] || { echo "dirty tree"; exit 3; }
git -C "$repo" checkout -q --detach main || exit 4
git -C "$repo" rev-parse HEAD > "$repo/private/cg22op/$before.main-head"
"$copy/build.sh" "$before"
built=$?
if [ "$built" = 0 ]; then
  "$copy/measure-binary.sh" "$before" "$repo/private/cg22op/bin/$before"
fi
git -C "$repo" checkout -q "$branch" || exit 5
echo "before built=$built; back on $(git -C "$repo" rev-parse --abbrev-ref HEAD)"
