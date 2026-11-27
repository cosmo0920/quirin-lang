#!/usr/bin/env bash
# build-before.sh NAME COMMIT -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: build
# a release quirin-audit from an export of COMMIT (`git archive`, with the
# libevent submodule's files copied in) in private/mpuh/NAME-src, so edits in
# the worktree never reach the measured binary while the build waits on the
# shared heavy lock. The export's compiled-in CARGO_MANIFEST_DIR reads the
# export's contracts. BEFORE is `before cd1239503` (the frozen prediction over
# main d708641bc: product code identical to main). Keeps a copy as
# private/mpuh/bin/NAME. Exits to files.
set -u
name="$1"; commit="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/mpuh"
src="$out/$name-src"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out/bin" "$src" || exit 2
if [ ! -f "$src/Cargo.toml" ]; then
  git -C "$repo" archive -o "$out/$name-src.tar" "$commit" || exit 2
  tar -xf "$out/$name-src.tar" -C "$src" || exit 2
  cp -r "$repo/third_party/libevent/." "$src/third_party/libevent/" || exit 2
fi
echo "$commit" > "$out/build-$name.head"
cd "$src" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit \
  --target-dir "$out/target-$name" > "$out/build-$name.log" 2>&1
echo $? > "$out/build-$name.exit"
[ "$(cat "$out/build-$name.exit")" = 0 ] || exit 1
cp "$out/target-$name/release/quirin-audit" "$out/bin/$name"
sha256sum "$out/bin/$name" > "$out/bin/$name.sha256"
cat "$out/bin/$name.sha256"
