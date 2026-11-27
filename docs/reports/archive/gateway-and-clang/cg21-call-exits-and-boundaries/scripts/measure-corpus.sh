#!/usr/bin/env bash
# measure-corpus.sh NAME -- CG21-CALL-EXITS-AND-BOUNDARIES: measure.sh's
# corpus arbiter alone, into private/cg21/NAME. `scripts/run_pinned_audit.py`
# always runs this worktree's target/release/quirin-audit (it takes no binary
# argument), so the binary is copied and digested first. Every exit is
# written to a file; nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg21"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
cp target/release/quirin-audit "$out/quirin-audit-$name" || exit 2
sha256sum "$out/quirin-audit-$name" > "$out/$name.binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" python3 scripts/run_pinned_audit.py \
  "$out/$name" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 4 > "$out/$name.log" 2>&1
echo $? > "$out/$name.exit"
