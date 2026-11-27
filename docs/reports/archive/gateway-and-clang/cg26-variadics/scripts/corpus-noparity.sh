#!/usr/bin/env bash
# corpus-noparity.sh NAME -- the corpus arbiter alone into private/cg26va/NAME, with no
# parity stream (two parity runs died of a full shared disk, ENOSPC at the
# parity flush; the arbiter result does not depend on the stream).
# scripts/run_pinned_audit.py always runs target/release/quirin-audit (it
# ignores measure.sh's BINARY), so the binary to measure must be installed
# there first; its sha256 is recorded beside the run.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg26va"
cd "$repo" || exit 2
sha256sum "$repo/target/release/quirin-audit" > "$out/$name.binary.sha256"
python3 scripts/run_pinned_audit.py \
  "$out/$name" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 4 > "$out/$name.log" 2>&1
echo $? > "$out/$name.exit"
cat "$out/$name.exit"
