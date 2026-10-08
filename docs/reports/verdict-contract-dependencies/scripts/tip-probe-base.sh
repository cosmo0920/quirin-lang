#!/usr/bin/env bash
# tip-probe-base.sh NAME BASE -- VERDICT-CONTRACT-DEPENDENCIES: tip-probe.sh with
# the base revision's librdkafka contracts in place (the BEFORE binary reads
# the worktree's contract files, and the base loader rejects the card's
# `resource_premise` clause). The two files are rewritten from BASE, the probe
# runs, and the card's bytes are written back and their digests asserted.
set -u
name="$1"; base="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
keep="$repo/private/vcd/tip-base-keep"
mkdir -p "$keep"
cd "$repo" || exit 2
files="providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv providers/qcore-provider-librdkafka/contracts.lock"
for f in $files; do
  cat "$f" > "$keep/$(basename "$f")" || exit 3
  sha256sum "$f" | cut -d' ' -f1 > "$keep/$(basename "$f").sha256"
  git show "$base:$f" > "$f" || exit 3
done
bash "$here/tip-probe.sh" "$name"
status=$?
for f in $files; do
  cat "$keep/$(basename "$f")" > "$f"
  touch "$f"
  [ "$(sha256sum "$f" | cut -d' ' -f1)" = "$(cat "$keep/$(basename "$f").sha256")" ] || { echo "restore failed: $f"; exit 4; }
done
echo "restored; probe exit $status"
