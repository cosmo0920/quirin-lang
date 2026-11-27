#!/usr/bin/env bash
# build-main.sh REV NAME -- HANDOFF-CANDIDATE-OWN-CALLBACKS: export REV with
# `git archive` into private/hcoc/NAME-src (its third_party/libevent submodule
# copied from this worktree's checkout) and build its release quirin-audit
# there under the shared heavy lock, keeping a copy as private/hcoc/bin/NAME.
# The binary reads REV's contracts (its compiled-in manifest path is the
# export). Exits to files; nothing piped.
set -u
rev="$1"; name="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/hcoc"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
src="$out/$name-src"
rm -rf "$src" && mkdir -p "$src" "$out/bin" || exit 2
cd "$repo" || exit 2
git archive "$rev" > "$out/$name.tar" || exit 3
tar -xf "$out/$name.tar" -C "$src" || exit 3
rm -f "$out/$name.tar"
# The libevent source pin needs a git checkout (a copy of the worktree's
# submodule keeps a relative gitdir that does not resolve from the export):
# clone the submodule's git directory and check out its pinned commit.
rm -rf "$src/third_party/libevent" || exit 3
git clone -q --no-checkout "$(git -C "$repo/third_party/libevent" rev-parse --absolute-git-dir)" \
  "$src/third_party/libevent" || exit 3
git -C "$src/third_party/libevent" checkout -q "$(git -C "$repo/third_party/libevent" rev-parse HEAD)" || exit 3
echo "$rev $(git rev-parse "$rev")" > "$out/build-$name.head"
# EXPORT_ONLY=1: recreate the export a saved binary reads (its contracts and
# libevent pin), without building.
if [ "${EXPORT_ONLY:-0}" = 1 ]; then exit 0; fi
cd "$src" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit \
  --target-dir "$out/target-$name" > "$out/build-$name.log" 2>&1
echo $? > "$out/build-$name.exit"
[ "$(cat "$out/build-$name.exit")" = 0 ] || exit 1
cp "$out/target-$name/release/quirin-audit" "$out/bin/$name"
sha256sum "$out/bin/$name" > "$out/bin/$name.sha256"
cat "$out/bin/$name.sha256"
