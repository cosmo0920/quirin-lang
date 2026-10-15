#!/usr/bin/env bash
# before.sh BASE -- MEMBER-PREPASS-OTHER-ROUTES (adapted from
# OP-CALLBACK-MEMBER-PATH-RELEASE and OP-APP-RK-CALLBACKS-VIA-POLL-CB): build
# the BEFORE binary from `git archive BASE` in private/mpor/base-src (so no
# edit of this worktree can reach it, and its compiled-in paths read the base
# tree's contracts and profile), keep it as private/mpor/bin/before, and
# measure it on every route with the null property. Each step's exit is
# written to a file; nothing is piped.
set -u
base="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/mpor"
mkdir -p "$out/bin" || exit 2
rm -rf "$out/base-src"
mkdir -p "$out/base-src" || exit 2
git -C "$repo" archive --format=tar --output="$out/base-src.tar" "$base" || exit 2
tar -xf "$out/base-src.tar" -C "$out/base-src" || exit 2
rm -f "$out/base-src.tar"
echo "$base" > "$out/base-src.revision"
REPO="$out/base-src" "$here/build.sh" before > "$out/before-build.out" 2>&1
echo $? > "$out/before-build.step.exit"
[ "$(cat "$out/before-build.step.exit")" = 0 ] || exit 1
cp "$out/base-src/private/mpor/bin/before" "$out/bin/before"
sha256sum "$out/bin/before" > "$out/bin/before.sha256"
NULL=1 "$here/measure-binary.sh" before "$out/bin/before" > "$out/before-measure.out" 2>&1
echo $? > "$out/before-measure.step.exit"
