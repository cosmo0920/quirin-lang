#!/usr/bin/env bash
# corpus.sh NAME -- the corpus arbiter alone with this worktree's release
# quirin-audit (as measure.sh runs it), into private/cg30dft/NAME. Used to
# re-run the corpus after a run lost to an exhausted disk.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg30dft"
cd "$repo" || exit 2
sha256sum "$repo/target/release/quirin-audit" > "$out/$name.binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" python3 scripts/run_pinned_audit.py \
  "$out/$name" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 4 > "$out/$name.log" 2>&1
echo $? > "$out/$name.exit"
