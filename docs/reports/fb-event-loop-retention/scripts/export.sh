#!/usr/bin/env bash
# export.sh NAME REV -- FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: a clean
# source tree of REV at private/fbel/NAME-src (git archive), with the pinned
# libevent submodule as a git clone (the corpus run verifies its revision
# through git), so a measurement binary is built from committed source only.
set -u
name="$1"; rev="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/fbel/$name-src"
cd "$repo" || exit 2
rm -rf "$out" && mkdir -p "$out" || exit 2
git archive --format=tar -o "$repo/private/fbel/$name.tar" "$rev" || exit 3
tar -xf "$repo/private/fbel/$name.tar" -C "$out" && rm "$repo/private/fbel/$name.tar"
rm -rf "$out/third_party/libevent"
if [ ! -d "$repo/private/fbel/libevent-clone/.git" ]; then
  git clone -q third_party/libevent "$repo/private/fbel/libevent-clone" || exit 4
fi
git -C "$repo/private/fbel/libevent-clone" checkout -q "$(git rev-parse HEAD:third_party/libevent)" || exit 5
cp -r "$repo/private/fbel/libevent-clone" "$out/third_party/libevent"
git rev-parse "$rev" > "$repo/private/fbel/$name-src.head"
cat "$repo/private/fbel/$name-src.head"
