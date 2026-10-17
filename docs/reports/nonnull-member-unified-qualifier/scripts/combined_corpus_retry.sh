#!/usr/bin/env bash
# combined_corpus_retry.sh MAINNAME -- NONNULL-MEMBER-UNIFIED-QUALIFIER: the
# combined arbiter's first main corpus run stopped at the libevent source pin
# (the export's third_party/libevent was a plain copy, so `git rev-parse` there
# answered this worktree's HEAD). With the export's libevent made a local clone
# checked out at the pinned d82464a2, rerun main's corpus route only; the first
# attempt is kept as MAINNAME-attempt1* and discarded.
set -u
mainname="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
src="$repo/private/nmuq/main-src"
cd "$repo/private/nmuq" || exit 2
mkdir -p "attempt1"
for f in "$mainname" "$mainname".exit "$mainname".log "$mainname".stderr "$mainname".seconds "$mainname"-null.jsonl "$mainname"-null.summary.json "$mainname".corpus-binary.sha256; do
  [ -e "$f" ] && mv "$f" "attempt1/"
done
mkdir -p "$src/private/nmuq/bin"
cp "bin/$mainname" "$src/private/nmuq/bin/$mainname"
( cd "$src" && bash docs/reports/nonnull-member-unified-qualifier/scripts/measure.sh "$mainname" "private/nmuq/bin/$mainname" corpus )
for f in "$src/private/nmuq/$mainname" "$src/private/nmuq/$mainname".* "$src/private/nmuq/$mainname"-null.*; do
  [ -e "$f" ] && mv "$f" "$repo/private/nmuq/"
done
rm -rf "$src/target"
echo done > "$repo/private/nmuq/combined-retry.done"
