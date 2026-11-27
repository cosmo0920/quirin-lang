#!/usr/bin/env bash
# corpus.sh NAME -- CG18-FOOTPRINT-CONNECTION: the corpus arbiter alone with the
# release binary in target/release, into a fresh private/cg18fc/NAME (used for
# the BEFORE on b59aaae2 after the first attempt's directory was left by a
# killed run; see the report). Every exit is written to a file.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg18fc"
cd "$repo" || exit 2
sha256sum "$repo/target/release/quirin-audit" > "$out/$name.binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" python3 scripts/run_pinned_audit.py \
  "$out/$name" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 4 > "$out/$name.log" 2>&1
echo $? > "$out/$name.exit"
