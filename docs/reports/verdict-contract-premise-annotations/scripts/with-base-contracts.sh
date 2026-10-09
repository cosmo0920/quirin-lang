#!/usr/bin/env bash
# with-base-contracts.sh BASE COMMAND... -- VERDICT-CONTRACT-PREMISE-ANNOTATIONS:
# run COMMAND with the base revision's contracts in place. The binary reads
# the worktree's contract files at run time and this card changes no code, so
# a BEFORE on a moved base is the rebased binary over BASE's six modules and
# three locks. They are rewritten from BASE, COMMAND runs, and the card's bytes
# are written back and their digests asserted (the restore fails loudly).
set -u
base="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
keep="$repo/private/vcpa/base-contracts-keep"
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
"$@"
status=$?
for f in $files; do
  k="$keep/$(echo "$f" | tr / _)"
  cat "$k" > "$f"
  touch "$f"
  [ "$(sha256sum "$f" | cut -d' ' -f1)" = "$(cat "$k.sha256")" ] || { echo "restore failed: $f"; exit 4; }
done
echo "restored; command exit $status"
