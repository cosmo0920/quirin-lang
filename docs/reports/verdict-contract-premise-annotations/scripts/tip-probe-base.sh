#!/usr/bin/env bash
# tip-probe-base.sh NAME BASE -- VERDICT-CONTRACT-PREMISE-ANNOTATIONS (from
# VERDICT-CONTRACT-DEPENDENCIES): tip-probe.sh with the base revision's
# contracts in place (the binary reads the worktree's contract files, and this
# card changes no binary). The card's six modules and three locks are
# rewritten from BASE, the probe runs, and the card's bytes are written back
# and their digests asserted.
set -u
name="$1"; base="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
keep="$repo/private/vcpa/tip-base-keep"
mkdir -p "$keep"
cd "$repo" || exit 2
files="providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv
providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv
providers/qcore-provider-librdkafka/contracts.lock
providers/quirin-provider-fluent-bit/contracts/fluent_bit/output_singleplex.qv
providers/quirin-provider-fluent-bit/contracts/fluent_bit/loop_owner.qv
providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_chunk_name.qv
providers/quirin-provider-fluent-bit/contracts.lock
providers/quirin-provider-libc/contracts/libc/errno.qv
providers/quirin-provider-libc/contracts.lock"
for f in $files; do
  k="$keep/$(echo "$f" | tr / _)"
  cat "$f" > "$k" || exit 3
  sha256sum "$f" | cut -d' ' -f1 > "$k.sha256"
  git show "$base:$f" > "$f" || exit 3
done
bash "$here/tip-probe.sh" "$name"
status=$?
for f in $files; do
  k="$keep/$(echo "$f" | tr / _)"
  cat "$k" > "$f"
  touch "$f"
  [ "$(sha256sum "$f" | cut -d' ' -f1)" = "$(cat "$k.sha256")" ] || { echo "restore failed: $f"; exit 4; }
done
echo "restored; probe exit $status"
