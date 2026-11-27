#!/usr/bin/env bash
# corpus.sh NAME -- CG16-INDEX-AND-OFFSET-EXTENTS: the corpus arbiter alone,
# with the bounds property selected, with whatever release quirin-audit is in
# target/release (scripts/run_pinned_audit.py always runs that path). Used for
# the AFTER corpus after the first attempt ran the BEFORE binary (see the
# report). Every exit is written to a file; nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg16ix"
cd "$repo" || exit 2
sha256sum target/release/quirin-audit > "$out/$name.binary.sha256"
QCORE_BOUNDS_PROPERTY="$out/$name-bounds.jsonl" \
  QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" python3 scripts/run_pinned_audit.py \
  "$out/$name" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 4 > "$out/$name.log" 2>&1
echo $? > "$out/$name.exit"
