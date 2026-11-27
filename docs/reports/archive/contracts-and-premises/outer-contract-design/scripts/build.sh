#!/usr/bin/env bash
# build.sh NAME -- OUTER-CONTRACT-DESIGN (from FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE, FB-QUANT-CONTRACT-CENSUS and
# NULL-TRIAGE-UNRESOLVED-PATHS's scripts/build.sh): under the shared heavy
# lock, build the release quirin-audit of the checked-out tree into a private
# target directory (private/ocd/target-rel), then keep a copy as
# private/ocd/bin/NAME with its sha256. Exits to files; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/ocd"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out/bin" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
git rev-parse HEAD > "$out/build-$name.head"
git status --porcelain --untracked-files=no > "$out/build-$name.dirty"
target="$out/target-rel"
flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit \
  --target-dir "$target" > "$out/build-$name.log" 2>&1
echo $? > "$out/build-$name.exit"
[ "$(cat "$out/build-$name.exit")" = 0 ] || exit 1
cp "$target/release/quirin-audit" "$out/bin/$name"
sha256sum "$out/bin/$name" > "$out/bin/$name.sha256"
cat "$out/bin/$name.sha256"
