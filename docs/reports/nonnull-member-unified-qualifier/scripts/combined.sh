#!/usr/bin/env bash
# combined.sh MAIN MAINNAME TIPNAME -- NONNULL-MEMBER-UNIFIED-QUALIFIER: the
# combined arbiter after the rebase. Export MAIN (`git archive`) into
# private/nmuq/main-src with the libevent submodule's files copied from this
# worktree (the same pinned revision), copy this card's build.sh and
# measure.sh into it, build main's release binary there and measure it on the
# corpus, system and slice routes with the null property on main's own
# contracts; then build this tip's binary and measure it here. Main's outputs
# are moved into private/nmuq/ for compare.py. Every heavy step takes the
# shared lock inside build.sh / measure.sh; release targets are deleted after.
set -u
main="$1"; mainname="$2"; tipname="$3"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
src="$repo/private/nmuq/main-src"
cd "$repo" || exit 2
rm -rf "$src" && mkdir -p "$src" || exit 2
git archive --format=tar "$main" > "$repo/private/nmuq/main.tar" || exit 3
tar -xf "$repo/private/nmuq/main.tar" -C "$src" || exit 3
rm -f "$repo/private/nmuq/main.tar"
mkdir -p "$src/third_party/libevent"
cp -r "$repo/third_party/libevent/." "$src/third_party/libevent/"
rm -rf "$src/third_party/libevent/.git"
echo "$main" > "$src/EXPORTED_FROM"
mkdir -p "$src/docs/reports/nonnull-member-unified-qualifier/scripts"
cp "$here/build.sh" "$here/measure.sh" "$src/docs/reports/nonnull-member-unified-qualifier/scripts/"
# build.sh records `git rev-parse HEAD`, which in the export is this worktree's
# HEAD: EXPORTED_FROM names the measured tree.
( cd "$src" && bash docs/reports/nonnull-member-unified-qualifier/scripts/build.sh "$mainname" ) || exit 4
( cd "$src" && bash docs/reports/nonnull-member-unified-qualifier/scripts/measure.sh "$mainname" "private/nmuq/bin/$mainname" corpus,system,slice )
rm -rf "$src/private/nmuq/target-rel"
for f in "$src/private/nmuq/$mainname"*; do mv "$f" "$repo/private/nmuq/"; done
mkdir -p "$repo/private/nmuq/bin"
mv "$src/private/nmuq/bin/$mainname" "$src/private/nmuq/bin/$mainname.sha256" "$repo/private/nmuq/bin/"
bash "$here/build.sh" "$tipname" || exit 6
bash "$here/measure.sh" "$tipname" "private/nmuq/bin/$tipname" corpus,system,slice
rm -rf "$repo/private/nmuq/target-rel" "$src/target"
echo done > "$repo/private/nmuq/combined-$tipname.done"
