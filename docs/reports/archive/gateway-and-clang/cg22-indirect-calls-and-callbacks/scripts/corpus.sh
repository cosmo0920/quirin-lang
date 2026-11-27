#!/usr/bin/env bash
# corpus.sh NAME -- the corpus arbiter alone (scripts/run_pinned_audit.py) with
# the release binary already in target/release, into a fresh private/cg22/NAME.
# Used to re-measure after attempt `after` was killed by the host disk filling
# (`No space left on device` in the parity sink; retained, not evidence).
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg22"
cd "$repo" || exit 2
sha256sum target/release/quirin-audit > "$out/$name.binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" python3 scripts/run_pinned_audit.py \
  "$out/$name" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 4 > "$out/$name.log" 2>&1
echo $? > "$out/$name.exit"
