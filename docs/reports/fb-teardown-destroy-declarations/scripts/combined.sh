#!/usr/bin/env bash
# combined.sh MAIN_REV -- FB-TEARDOWN-DESTROY-DECLARATIONS: the combined
# arbiter. MAIN_REV is exported clean (`git archive`, libevent submodule
# cloned from this worktree's checkout at its pinned revision) into
# private/ftd/src-combined, its quirin-audit built there and measured on the
# corpus, system and slice routes with the null property from the export
# (the export's contracts); then this worktree's tip is built
# (scripts/build.sh tipc) and measured the same way (the tip's contracts).
# Every build and route runs under the shared heavy lock (build.sh,
# measure.sh); exits to files; nothing piped.
set -u
rev="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
src="$repo/private/ftd/src-combined"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo" || exit 2
rm -rf "$src"
mkdir -p "$src" || exit 2
git archive "$rev" | tar -x -C "$src" || exit 3
rmdir "$src/third_party/libevent" 2>/dev/null
git clone -q "$repo/third_party/libevent" "$src/third_party/libevent" || exit 3
git -C "$src/third_party/libevent" checkout -q "$(git rev-parse "$rev:third_party/libevent")" || exit 3
git rev-parse "$rev" > "$repo/private/ftd/combined-main.rev"
mkdir -p "$src/private/ftd/bin" "$src/docs/reports/fb-teardown-destroy-declarations/scripts" || exit 2
cp "$here/measure.sh" "$src/docs/reports/fb-teardown-destroy-declarations/scripts/measure.sh" || exit 2
(
  cd "$src" || exit 2
  export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
  flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit \
    > "$src/private/ftd/build-mainc.log" 2>&1
  echo $? > "$src/private/ftd/build-mainc.exit"
)
[ "$(cat "$src/private/ftd/build-mainc.exit")" = 0 ] || exit 1
cp "$src/target/release/quirin-audit" "$src/private/ftd/bin/mainc"
sha256sum "$src/private/ftd/bin/mainc" > "$src/private/ftd/bin/mainc.sha256"
rm -rf "$src/target"
bash "$src/docs/reports/fb-teardown-destroy-declarations/scripts/measure.sh" mainc \
  "$src/private/ftd/bin/mainc" system,slice,corpus
bash "$here/build.sh" tipc || exit 1
rm -rf "$repo/private/ftd/target-rel"
bash "$here/measure.sh" tipc "$repo/private/ftd/bin/tipc" system,slice,corpus
echo "combined done"
