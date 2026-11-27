#!/bin/sh
# CONTRACT-SUPERSESSION identity run: one source snapshot per side, as
# AUDIT-COMPOSITION took them (docs/reports/audit-composition/README.md):
# `git archive` of the revision, plus third_party/libevent cloned at its pin,
# under WORKTREE/private/cs/<side>-src, and the common mount point side-src.
# Usage: snapshot.sh WORKTREE SIDE REVISION   (SIDE: base|tip)
set -eu
W=$1
side=$2
rev=$3
A=$W/private/cs
dest=$A/$side-src
mkdir -p "$A/side-src"
rm -rf "$dest"
mkdir -p "$dest"
git -C "$W" archive "$rev" | tar -x -C "$dest"
rmdir "$dest/third_party/libevent" 2>/dev/null || true
git clone --quiet "$W/third_party/libevent" "$dest/third_party/libevent"
git -C "$dest/third_party/libevent" checkout --quiet d82464a277d0f42703702c4dfd9af6af38595a83
git -C "$W" rev-parse "$rev" >"$A/$side-rev"
