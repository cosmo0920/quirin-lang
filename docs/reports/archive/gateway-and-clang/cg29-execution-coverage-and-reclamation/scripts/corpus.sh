#!/usr/bin/env bash
# corpus.sh NAME BINARY -- CG29-EXECUTION-COVERAGE-AND-RECLAMATION's corpus arbiter run alone.
# scripts/run_pinned_audit.py always runs target/release/quirin-audit, so a
# binary built elsewhere (the AFTER, in target/after while the BEFORE was still
# measuring) is installed there first; run.json records the binary's sha256.
# measure.sh's BINARY override reaches only the system and slice runs: its
# first AFTER corpus ran the BEFORE binary (kept as `before-repeat`, a repeat
# of the BEFORE). Every exit is written to a file; nothing is piped.
set -u
name="$1"; binary="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg29"
cd "$repo" || exit 2
if [ "$(realpath "$binary")" != "$(realpath target/release/quirin-audit)" ]; then
  cp "$binary" target/release/quirin-audit || exit 2
fi
sha256sum target/release/quirin-audit > "$out/$name.binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" python3 scripts/run_pinned_audit.py \
  "$out/$name" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 4 > "$out/$name.log" 2>&1
echo $? > "$out/$name.exit"
